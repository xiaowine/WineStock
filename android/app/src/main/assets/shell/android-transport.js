// 本文件是 Android shell 注入 WebView 的 Shell Bridge 传输 shim；它属于 Android 平台传输层，
// 不是前端源码。它把 frontend/src/shell/contract.ts 约定的 v1 逻辑接口映射到原生消息通道，
// 不实现业务 HTTP、不持久化配置、也不解释业务 DTO。
//
// 传输模型（对齐 androidx.webkit WebMessageListener）：
//   - JS -> Native：channel.postMessage(JSON) 发送请求信封 { type:"call", id, method, params }。
//   - Native -> JS：channel.onmessage 收到回复信封 { type:"reply", id, ok, result?, error? }
//     或事件信封 { type:"event", event, payload? }（事件无 id）。
//
// 设备元数据（clientKind/deviceName/appVersion/channelName）由原生在本脚本前定义为
// window.__WINESTOCK_BRIDGE_META__，因此本文件保持为合法且可独立维护的 JS。
(function () {
  "use strict";

  // 幂等：文档已注入过桥时直接返回，避免重复覆盖 pending 状态。
  if (window.__WINESTOCK_SHELL_BRIDGE__) {
    return;
  }

  var META =
    typeof window.__WINESTOCK_BRIDGE_META__ === "object" &&
    window.__WINESTOCK_BRIDGE_META__
      ? window.__WINESTOCK_BRIDGE_META__
      : {};

  var channelName =
    typeof META.channelName === "string" && META.channelName
      ? META.channelName
      : "__winestockShellBridgeNative__";

  // 由原生登录元数据派生前端运行配置注入对象；不设置 apiBaseUrl，实际访问地址以 Shell 快照为准。
  window.__WINESTOCK_RUNTIME_CONFIG__ = {
    clientKind: "android",
    deviceName:
      typeof META.deviceName === "string"
        ? META.deviceName
        : "WineStock Android",
    appVersion:
      typeof META.appVersion === "string" ? META.appVersion : "unknown",
  };

  var channel = window[channelName];

  // WebMessageListener 注入的通道对象应在 document-start 脚本执行前就绪；缺失说明当前 WebView
  // 不满足桥的前置能力，此时把全部方法暴露为稳定失败，前端据此进入可修复的失败态。
  if (!channel || typeof channel.postMessage !== "function") {
    window.__WINESTOCK_SHELL_BRIDGE__ = createUnavailableBridge();
    return;
  }

  var nextRequestId = 1;
  var pendingRequests = Object.create(null);
  var runtimeStateListeners = new Set();
  var appResumedListeners = new Set();
  var nativeBackListeners = new Set();

  channel.onmessage = function (event) {
    var envelope = parseEnvelope(event && event.data);
    if (!envelope) {
      return;
    }
    if (envelope.type === "reply") {
      settleReply(envelope);
    } else if (envelope.type === "event") {
      dispatchEvent(envelope);
    }
  };

  function settleReply(envelope) {
    var id = envelope.id;
    var pending = id != null ? pendingRequests[id] : undefined;
    if (!pending) {
      return;
    }
    delete pendingRequests[id];
    if (envelope.ok) {
      pending.resolve(envelope.result);
    } else {
      pending.reject(toBridgeError(envelope.error));
    }
  }

  function dispatchEvent(envelope) {
    if (envelope.event === "runtimeStateChanged") {
      runtimeStateListeners.forEach(function (listener) {
        safeInvoke(listener, envelope.payload);
      });
    } else if (envelope.event === "appResumed") {
      appResumedListeners.forEach(function (listener) {
        safeInvoke(listener);
      });
    } else if (envelope.event === "nativeBackRequested") {
      nativeBackListeners.forEach(function (listener) {
        safeInvoke(listener, envelope.payload);
      });
    }
  }

  // 发送一次请求并按 id 匹配回复；原生错误还原为带 code 的 Error，供前端映射稳定错误码。
  function call(method, params) {
    return new Promise(function (resolve, reject) {
      var id = nextRequestId++;
      pendingRequests[id] = { resolve: resolve, reject: reject };
      var payload = { type: "call", id: id, method: method };
      if (params !== undefined) {
        payload.params = params;
      }
      try {
        channel.postMessage(JSON.stringify(payload));
      } catch (error) {
        delete pendingRequests[id];
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  function subscribe(listenerSet, listener) {
    listenerSet.add(listener);
    return Promise.resolve(function () {
      listenerSet.delete(listener);
    });
  }

  window.__WINESTOCK_SHELL_BRIDGE__ = {
    getRuntimeSnapshot: function () {
      return call("getRuntimeSnapshot");
    },
    validateRuntimeConfig: function (config) {
      return call("validateRuntimeConfig", { config: config });
    },
    applyRuntimeConfig: function (config) {
      return call("applyRuntimeConfig", { config: config });
    },
    startLocalService: function () {
      return call("startLocalService");
    },
    stopLocalService: function () {
      return call("stopLocalService");
    },
    restartLocalService: function () {
      return call("restartLocalService");
    },
    frontendReady: function () {
      return call("frontendReady");
    },
    reportFrontendFailure: function (message) {
      return call("frontendFailed", { message: message });
    },
    openExternal: function (url) {
      return call("openExternal", { url: url });
    },
    resolveNativeBack: function (resolution) {
      return call("resolveNativeBack", resolution);
    },
    onRuntimeStateChanged: function (listener) {
      return subscribe(runtimeStateListeners, listener);
    },
    onAppResumed: function (listener) {
      return subscribe(appResumedListeners, listener);
    },
    onNativeBackRequested: function (listener) {
      return subscribe(nativeBackListeners, listener);
    },
  };

  function parseEnvelope(data) {
    if (typeof data !== "string") {
      return null;
    }
    try {
      var value = JSON.parse(data);
      return value && typeof value === "object" ? value : null;
    } catch (error) {
      return null;
    }
  }

  function toBridgeError(error) {
    var message =
      error && typeof error.message === "string"
        ? error.message
        : "Shell Bridge 调用失败";
    var bridgeError = new Error(message);
    if (error && typeof error.code === "string") {
      bridgeError.code = error.code;
    }
    return bridgeError;
  }

  function safeInvoke(listener, argument) {
    try {
      listener(argument);
    } catch (error) {
      // 单个订阅者异常不应中断其它订阅者或原生事件循环。
      if (typeof console !== "undefined" && console.warn) {
        console.warn("Shell Bridge 事件订阅回调异常", error);
      }
    }
  }

  // WebView 不支持消息通道时使用的降级桥：读取类操作返回失败态，写入类操作明确拒绝。
  function createUnavailableBridge() {
    var reason = "当前 WebView 不支持 WineStock Shell Bridge 消息通道";
    var rejected = function () {
      return Promise.reject(new Error(reason));
    };
    return {
      getRuntimeSnapshot: rejected,
      validateRuntimeConfig: rejected,
      applyRuntimeConfig: rejected,
      startLocalService: rejected,
      stopLocalService: rejected,
      restartLocalService: rejected,
      frontendReady: function () {
        return Promise.resolve();
      },
      reportFrontendFailure: function () {
        return Promise.resolve();
      },
      openExternal: rejected,
      resolveNativeBack: rejected,
      onRuntimeStateChanged: function () {
        return Promise.resolve(function () {});
      },
      onAppResumed: function () {
        return Promise.resolve(function () {});
      },
      onNativeBackRequested: function () {
        return Promise.resolve(function () {});
      },
    };
  }
})();
