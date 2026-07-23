# Android WebView 文件选择与权限提示

> 本文是通用实现提示，不是特定应用的功能契约。实际实现仍需结合应用的
> `minSdk`、`targetSdk`、Android System WebView 版本、文件用途和上架政策核对。

## 快速结论

网页在 Android WebView 中通过 `<input type="file">` 选择文件时，如果宿主应用使用系统文件选择器、
Storage Access Framework 或系统 Photo Picker，通常不需要申请存储或媒体库权限。

系统会针对用户明确选择的 `content://` URI 授予临时读取权限。因此，不应仅为了网页上传文件而申请：

- `READ_EXTERNAL_STORAGE`
- `WRITE_EXTERNAL_STORAGE`
- `READ_MEDIA_IMAGES`
- `READ_MEDIA_VIDEO`
- `READ_MEDIA_AUDIO`
- `MANAGE_EXTERNAL_STORAGE`

只有应用绕过系统选择器、主动扫描共享媒体库，或直接使用摄像头、麦克风等设备能力时，才需要按实际能力和
Android 版本申请对应权限。

## WebView 宿主需要承担的职责

Android WebView 不会替宿主完整实现网页文件选择流程。宿主通常需要：

1. 在 `WebChromeClient.onShowFileChooser()` 中接收 `ValueCallback<Array<Uri>>`。
2. 根据 `FileChooserParams` 启动系统选择器，或按产品需要构造受控的自定义 chooser。
3. 在 Activity Result 返回后，把 URI 数组交回 WebView。
4. 用户取消、Activity 销毁或新请求覆盖旧请求时，以 `null` 结束旧回调。
5. 使用 `ContentResolver` 读取 `content://` URI，不把它强制转换成所谓“真实文件路径”。

`FileChooserParams.createIntent()` 适合基础文件选择。需要拍照、录像、目录选择或复杂来源组合时，宿主可能需要
自行构造 Intent，并分别处理选择结果和拍摄结果。

系统 Photo Picker 首先随 Android 13 提供，并可通过系统组件或 Google Play 服务覆盖部分较早版本。设备支持情况
可能不同，建议使用 AndroidX Activity Result 的视觉媒体选择契约及其可用性判断，让不支持 Photo Picker 的设备
回退到 `ACTION_OPEN_DOCUMENT`，不要只按 Android 版本号假定选择器一定存在。

## 常见场景与权限

| 场景                                         | 通常需要的 Android 权限       | 说明                                                        |
| -------------------------------------------- | ----------------------------- | ----------------------------------------------------------- |
| 系统文件选择器选择文档                       | 无存储权限                    | 通过返回 URI 的临时授权读取所选文件。                       |
| 系统 Photo Picker 选择照片或视频             | 无存储权限                    | 用户只向应用共享明确选择的媒体。                            |
| 读取应用内部目录或应用专属外部目录           | 无存储权限                    | 仅限应用自己的目录。                                        |
| 应用自行枚举整个共享相册                     | 取决于 Android 版本           | 需要旧版存储读取权限或 Android 13+ 的细粒度媒体权限。       |
| 通过外部相机应用拍摄并返回文件               | 通常不需要 `CAMERA`           | 使用拍摄 Intent 和 `FileProvider` URI 临时授权。            |
| 应用直接使用 Camera2、CameraX 等相机 API     | `CAMERA`                      | Android 6.0+ 还需要运行时授权。                             |
| 应用直接录音或为实时视频采集声音             | `RECORD_AUDIO`                | 与普通文件选择不是同一权限流程。                            |
| 网页使用 `getUserMedia()` 获取摄像头或麦克风 | `CAMERA` 和/或 `RECORD_AUDIO` | 还需由 WebView 宿主处理可信来源的 `onPermissionRequest()`。 |
| 文件管理器等特殊应用访问全部共享文件         | `MANAGE_EXTERNAL_STORAGE`     | 受系统和应用商店政策严格限制，不适用于普通网页上传。        |

## Android 版本差异

以下差异主要影响“应用主动读取共享存储或媒体库”的实现。通过系统选择器读取用户所选 URI 时，各版本均不应
为了方便而额外申请广泛存储权限。

| Android 版本                   | 主动读取共享内容时的典型规则                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Android 5.1 及以下（API 22-）  | 旧权限在安装时授予；系统选择器返回的 URI 仍可依赖 URI 授权读取。                                                                                  |
| Android 6.0 至 9（API 23–28）  | 主动读取共享存储通常使用 `READ_EXTERNAL_STORAGE`，且属于运行时权限；旧式直接写入可能使用 `WRITE_EXTERNAL_STORAGE`。                               |
| Android 10（API 29）           | 引入分区存储。兼容开关只适合作为迁移手段，新实现应使用 MediaStore、Storage Access Framework 或应用专属目录。                                      |
| Android 11 至 12L（API 30–32） | 面向现代 target SDK 时强制分区存储；`WRITE_EXTERNAL_STORAGE` 不再扩大访问范围。主动读取其他应用创建的共享媒体仍可能需要 `READ_EXTERNAL_STORAGE`。 |
| Android 13（API 33）           | 面向 API 33+ 时，主动读取共享媒体改用 `READ_MEDIA_IMAGES`、`READ_MEDIA_VIDEO`、`READ_MEDIA_AUDIO`。Photo Picker 仍不需要这些权限。                |
| Android 14 及以上（API 34+）   | 自建相册选择界面时需要处理“仅选择的照片和视频”访问模式，可涉及 `READ_MEDIA_VISUAL_USER_SELECTED`；系统 Photo Picker 不需要该权限。                |

权限行为同时受设备 Android 版本和应用 `targetSdk` 影响。维护旧应用时应核对对应 target SDK 的兼容行为；新应用
不应依赖旧 target SDK 来保留宽泛存储访问。

## 系统选择器、URI 授权与长期读取

### 临时读取

`ACTION_GET_CONTENT`、系统 Photo Picker 等流程通常返回带有临时读取授权的 URI，足以完成即时预览和上传。
应用应直接打开 URI 输入流，不假设文件一定存在于本地磁盘，也不假设 URI 永久有效。

### 持久读取

如果文件需要跨进程重启、延迟任务或长期保存引用，应优先使用支持持久授权的
`ACTION_OPEN_DOCUMENT`，并在返回结果允许时调用 `takePersistableUriPermission()`。

持久 URI 授权不是文件备份。来源文档仍可能被用户移动、删除，或因文档提供方变化而无法访问，读取失败必须作为
正常状态处理。

## 拍照与录像的区别

### 启动外部相机应用

仅通过 `ACTION_IMAGE_CAPTURE` 或 `ACTION_VIDEO_CAPTURE` 启动已安装的相机应用时，宿主通常不需要自己申请
`CAMERA`。输出文件应使用 `FileProvider` 生成的 `content://` URI，并向相机应用授予必要的临时读写权限。

如果应用因为其它功能已经在 Manifest 中声明 `CAMERA`，Android 6.0+ 上调用拍摄能力前可能必须先取得该运行时
权限。因此，只使用外部相机 Intent 的应用不要无故声明 `CAMERA`。

### 应用或网页直接采集

应用直接使用 Camera2、CameraX，或者网页通过 `getUserMedia()` 请求摄像头、麦克风时，是设备能力授权，不是
文件选择授权：

- 原生层需要申请 `CAMERA` 和/或 `RECORD_AUDIO` 运行时权限。
- WebView 还会通过 `WebChromeClient.onPermissionRequest()` 请求网页来源授权。
- 宿主只应向受信任来源授予其明确请求且原生权限已经获准的资源，不应直接执行无条件 `grant()`。

HTML 文件输入的 `capture` 属性只是来源偏好提示，不保证系统一定直接打开相机。最终行为由 WebView、宿主实现、
设备能力和已安装的处理应用共同决定。

## 何时才需要媒体读取权限

只有应用需要自行查询 MediaStore、展示自建的完整媒体网格、批量扫描或后台处理未由用户逐项选择的媒体时，才考虑
声明媒体读取权限。例如，面向 Android 13+ 的直接媒体访问可能按需要声明：

```xml
<!-- 仅在应用主动读取整个共享媒体库时声明，不是 WebView 文件选择的默认配置。 -->
<uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />
<uses-permission android:name="android.permission.READ_MEDIA_VIDEO" />
<uses-permission android:name="android.permission.READ_MEDIA_AUDIO" />
```

兼容 Android 12L 及以下且确实需要主动读取共享媒体时，可能还需要：

```xml
<!-- 限制到旧 Android，避免在新版本继续请求已经被替代的权限。 -->
<uses-permission
    android:name="android.permission.READ_EXTERNAL_STORAGE"
    android:maxSdkVersion="32" />
```

不要复制未被真实功能使用的权限。图片、视频和音频也应按实际类型分别声明，而不是默认全部申请。

## 安全与可靠性提示

- `accept`、扩展名、文件名和提供方返回的 MIME 类型都只能作为提示；服务端仍需校验实际内容、大小和格式。
- 不要因为部分设备或文件管理器兼容性问题，直接升级为 `MANAGE_EXTERNAL_STORAGE`。
- 不要依赖 `_data` 列或文件路径反查 `content://` URI；云盘、相册和远程文档可能没有可用本地路径。
- 正确处理多选 `ClipData`、用户取消、重复打开选择器、Activity 重建和回调清理。
- 自定义相机输出 URI 时使用 `FileProvider`，不要向其它应用暴露 `file://` URI。
- 网页来源不可信时，不应开放相机、麦克风或任意原生文件能力。
- 如果只需要用户挑选少量照片或视频，优先使用 Photo Picker，减少权限请求和隐私暴露面。

## 建议验收项

- 未授予任何存储权限时，普通文档、图片和视频仍能选择并上传。
- Android 12L、Android 13、Android 14+ 至少各覆盖一个代表版本。
- 单选、多选、取消、重新选择和 Activity 重建不会遗留旧回调。
- 能读取来自本地文件、相册、下载目录和云文档提供方的 `content://` URI。
- 应用被置于后台或重启后，长期任务使用的 URI 授权行为符合预期。
- 拍摄取消、相机应用不存在、输出文件创建失败时能够正确结束 WebView 回调。
- `getUserMedia()` 只对明确允许的可信来源开放，拒绝时网页能够正常降级。
- Manifest 中没有与实际功能无关的存储、媒体、相机或全文件访问权限。

## 官方参考

- [WebChromeClient.onShowFileChooser](<https://developer.android.com/reference/android/webkit/WebChromeClient#onShowFileChooser(android.webkit.WebView,android.webkit.ValueCallback%3Candroid.net.Uri%5B%5D%3E,android.webkit.WebChromeClient.FileChooserParams)>)
- [WebChromeClient.FileChooserParams](https://developer.android.com/reference/android/webkit/WebChromeClient.FileChooserParams)
- [Storage Access Framework：访问文档和其它文件](https://developer.android.com/training/data-storage/shared/documents-files)
- [Android Photo Picker](https://developer.android.com/training/data-storage/shared/photopicker)
- [访问共享媒体文件](https://developer.android.com/training/data-storage/shared/media)
- [Android 13 细粒度媒体权限](https://developer.android.com/about/versions/13/behavior-changes-13#granular-media-permissions)
- [Android 14 部分照片和视频访问](https://developer.android.com/about/versions/14/changes/partial-photo-video-access)
- [相机 Intent](https://developer.android.com/media/camera/camera-intents)
- [FileProvider](https://developer.android.com/reference/androidx/core/content/FileProvider)
- [WebChromeClient.onPermissionRequest](<https://developer.android.com/reference/android/webkit/WebChromeClient#onPermissionRequest(android.webkit.PermissionRequest)>)
- [管理全部文件访问权限](https://developer.android.com/training/data-storage/manage-all-files)
