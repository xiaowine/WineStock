//! Desktop server-mode 的 Windows 防火墙协调器与受限提权入口。
//!
//! 本模块只管理 WineStock 自有的当前 TCP 端口规则，不代理业务请求；Windows 主进程保持普通用户权限，
//! 需要写规则时以固定参数启动同一签名程序的 helper 分支。

use crate::contract::{RuntimeFirewallSnapshot, ShellRuntimeError};

const RULE_NAME: &str = "WineStock Server Mode LAN Access";
const RULE_GROUP: &str = "WineStock";
const RULE_DESCRIPTION: &str = "WineStock server-mode local subnet access";
const SCOPE_LOCAL_SUBNET: &str = "local-subnet";

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::{env, ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt};

    use windows::{
        core::{BSTR, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, ERROR_ACCESS_DENIED, ERROR_CANCELLED},
            NetworkManagement::WindowsFirewall::{
                INetFwPolicy2, INetFwRule, NetFwPolicy2, NetFwRule, NET_FW_ACTION_ALLOW,
                NET_FW_IP_PROTOCOL_TCP, NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE,
                NET_FW_RULE_DIR_IN,
            },
            System::{
                Com::{
                    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                    COINIT_MULTITHREADED,
                },
                Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
            },
            UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
        },
    };

    const HELPER_FLAG: &str = "--winestock-firewall-helper";
    const ACTION_ENSURE: &str = "ensure";
    const ACTION_REMOVE: &str = "remove";
    const EXIT_OK: u32 = 0;
    const EXIT_AUTHORIZATION_REQUIRED: u32 = 10;
    const EXIT_POLICY_BLOCKED: u32 = 11;
    const EXIT_PROFILE_UNSUPPORTED: u32 = 12;
    const EXIT_SERVICE_UNAVAILABLE: u32 = 13;
    const EXIT_RULE_UPDATE_FAILED: u32 = 20;

    enum Probe {
        Ready,
        RequiresElevation,
        ProfileUnsupported,
        Disabled,
    }

    struct ComGuard;

    impl ComGuard {
        fn initialize() -> Result<Self, windows::core::Error> {
            // COM must be initialized on every thread that touches the Firewall interfaces.
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
            Ok(Self)
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    fn policy() -> windows::core::Result<INetFwPolicy2> {
        unsafe { CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER) }
    }

    fn active_profiles(policy: &INetFwPolicy2) -> windows::core::Result<i32> {
        unsafe { policy.CurrentProfileTypes() }
    }

    fn active_supported_profiles(policy: &INetFwPolicy2) -> windows::core::Result<i32> {
        let profiles = active_profiles(policy)?;
        Ok(profiles & (NET_FW_PROFILE2_DOMAIN.0 | NET_FW_PROFILE2_PRIVATE.0))
    }

    fn firewall_enabled_for_profiles(
        policy: &INetFwPolicy2,
        profiles: i32,
    ) -> windows::core::Result<bool> {
        for profile in [NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE] {
            if profiles & profile.0 != 0 {
                let enabled = unsafe { policy.get_FirewallEnabled(profile)? };
                if enabled.0 == 0 {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn rule_matches(rule: &INetFwRule, port: u16) -> windows::core::Result<bool> {
        let expected_profiles = NET_FW_PROFILE2_DOMAIN.0 | NET_FW_PROFILE2_PRIVATE.0;
        Ok(unsafe {
            rule.Protocol()? == NET_FW_IP_PROTOCOL_TCP.0
                && rule.LocalPorts()?.to_string() == port.to_string()
                && rule.RemoteAddresses()?.to_string() == "LocalSubnet"
                && rule.Profiles()? == expected_profiles
                && rule.Direction()? == NET_FW_RULE_DIR_IN
                && rule.Action()? == NET_FW_ACTION_ALLOW
                && rule.Enabled()?.0 != 0
                && rule.EdgeTraversal()?.0 == 0
                && rule.Grouping()?.to_string() == RULE_GROUP
        })
    }

    fn probe_with_policy(policy: &INetFwPolicy2, port: u16) -> windows::core::Result<Probe> {
        let profiles = active_supported_profiles(policy)?;
        if profiles == 0 {
            return Ok(Probe::ProfileUnsupported);
        }
        if !firewall_enabled_for_profiles(policy, profiles)? {
            return Ok(Probe::Disabled);
        }

        let rules = unsafe { policy.Rules()? };
        let name = BSTR::from(RULE_NAME);
        match unsafe { rules.Item(&name) } {
            Ok(rule) if rule_matches(&rule, port)? => Ok(Probe::Ready),
            Ok(_) | Err(_) => Ok(Probe::RequiresElevation),
        }
    }

    fn snapshot(status: &str, port: u16) -> RuntimeFirewallSnapshot {
        RuntimeFirewallSnapshot {
            status: status.to_owned(),
            port: Some(port),
            scope: Some(SCOPE_LOCAL_SUBNET.to_owned()),
        }
    }

    pub(crate) fn probe(port: u16) -> Result<RuntimeFirewallSnapshot, ShellRuntimeError> {
        let _com =
            ComGuard::initialize().map_err(|_| firewall_error("无法初始化 Windows 防火墙组件"))?;
        let current_policy =
            policy().map_err(|error| map_policy_error(error, "无法读取 Windows 防火墙策略"))?;
        match probe_with_policy(&current_policy, port) {
            Ok(Probe::Ready) => Ok(snapshot("ready", port)),
            Ok(Probe::ProfileUnsupported) => Ok(snapshot("profile-unsupported", port)),
            Ok(Probe::Disabled) => Ok(snapshot("disabled", port)),
            Ok(Probe::RequiresElevation) => Ok(snapshot("requires-elevation", port)),
            Err(error) => return Err(map_policy_error(error, "无法读取 WineStock 防火墙规则")),
        }
    }

    pub(crate) fn ensure(port: u16) -> Result<RuntimeFirewallSnapshot, ShellRuntimeError> {
        match probe(port)? {
            status if status.status != "requires-elevation" => return Ok(status),
            _ => {}
        }

        let exit_code = launch_helper(ACTION_ENSURE, port)?;
        match exit_code {
            EXIT_OK => {
                let status = probe(port)?;
                if status.status == "ready" {
                    Ok(status)
                } else {
                    Err(firewall_error("Windows 防火墙规则未生效"))
                }
            }
            EXIT_AUTHORIZATION_REQUIRED => Err(ShellRuntimeError::new(
                crate::contract::ERROR_FIREWALL_AUTHORIZATION_REQUIRED,
                "需要允许 Windows 防火墙访问局域网连接",
            )),
            EXIT_POLICY_BLOCKED => Err(ShellRuntimeError::new(
                crate::contract::ERROR_FIREWALL_POLICY_BLOCKED,
                "系统策略阻止修改 Windows 防火墙规则",
            )),
            EXIT_PROFILE_UNSUPPORTED => Ok(snapshot("profile-unsupported", port)),
            EXIT_SERVICE_UNAVAILABLE => Err(ShellRuntimeError::new(
                crate::contract::ERROR_FIREWALL_SERVICE_UNAVAILABLE,
                "Windows 防火墙服务不可用",
            )),
            _ => Err(firewall_error("Windows 防火墙规则更新失败")),
        }
    }

    pub(crate) fn remove(_port: u16) -> Result<(), ShellRuntimeError> {
        let _com =
            ComGuard::initialize().map_err(|_| firewall_error("无法初始化 Windows 防火墙组件"))?;
        let policy =
            policy().map_err(|error| map_policy_error(error, "无法读取 Windows 防火墙策略"))?;
        let rules = unsafe { policy.Rules() }
            .map_err(|error| map_policy_error(error, "无法读取防火墙规则集合"))?;
        let name = BSTR::from(RULE_NAME);
        let Ok(rule) = (unsafe { rules.Item(&name) }) else {
            return Ok(());
        };
        // 固定名称和分组标识 WineStock 自有规则。不能要求端口匹配：端口更新
        // 被拒绝后规则仍指向旧端口，离开 server-mode 时仍必须清理它。
        let owned = unsafe { rule.Grouping() }
            .map(|group| group.to_string() == RULE_GROUP)
            .map_err(|error| map_policy_error(error, "无法读取 WineStock 防火墙规则所有权"))?;
        if !owned {
            return Ok(());
        }
        drop(rule);
        let exit_code = launch_helper(ACTION_REMOVE, _port)?;
        if exit_code == EXIT_OK {
            Ok(())
        } else {
            Err(ShellRuntimeError::new(
                crate::contract::ERROR_FIREWALL_CLEANUP_PENDING,
                "Windows 防火墙规则清理未完成",
            ))
        }
    }

    fn helper_ensure(port: u16) -> u32 {
        let Ok(_com) = ComGuard::initialize() else {
            return EXIT_RULE_UPDATE_FAILED;
        };
        let Ok(policy) = policy() else {
            return EXIT_SERVICE_UNAVAILABLE;
        };
        let Ok(profiles) = active_supported_profiles(&policy) else {
            return EXIT_RULE_UPDATE_FAILED;
        };
        if profiles == 0 {
            return EXIT_PROFILE_UNSUPPORTED;
        }
        if firewall_enabled_for_profiles(&policy, profiles) == Ok(false) {
            return EXIT_SERVICE_UNAVAILABLE;
        }
        let Ok(rules) = (unsafe { policy.Rules() }) else {
            return EXIT_RULE_UPDATE_FAILED;
        };
        let name = BSTR::from(RULE_NAME);
        let rule = match unsafe { rules.Item(&name) } {
            Ok(rule) => rule,
            Err(_) => {
                let Ok(rule) =
                    (unsafe { CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER) })
                else {
                    return EXIT_RULE_UPDATE_FAILED;
                };
                if let Err(error) = configure_rule(&rule, port) {
                    return map_rule_update_error(error);
                }
                if let Err(error) = unsafe { rules.Add(&rule) } {
                    return map_rule_update_error(error);
                }
                return EXIT_OK;
            }
        };
        if let Err(error) = configure_rule(&rule, port) {
            return map_rule_update_error(error);
        }
        EXIT_OK
    }

    fn configure_rule(rule: &INetFwRule, port: u16) -> windows::core::Result<()> {
        let name = BSTR::from(RULE_NAME);
        let description = BSTR::from(RULE_DESCRIPTION);
        let group = BSTR::from(RULE_GROUP);
        let local_ports = BSTR::from(port.to_string());
        let remote_addresses = BSTR::from("LocalSubnet");
        unsafe {
            rule.SetName(&name)?;
            rule.SetDescription(&description)?;
            rule.SetGrouping(&group)?;
            rule.SetProtocol(NET_FW_IP_PROTOCOL_TCP.0)?;
            rule.SetLocalPorts(&local_ports)?;
            rule.SetRemoteAddresses(&remote_addresses)?;
            rule.SetProfiles(NET_FW_PROFILE2_DOMAIN.0 | NET_FW_PROFILE2_PRIVATE.0)?;
            rule.SetDirection(NET_FW_RULE_DIR_IN)?;
            rule.SetAction(NET_FW_ACTION_ALLOW)?;
            rule.SetEdgeTraversal(windows::Win32::Foundation::VARIANT_BOOL(0))?;
            rule.SetEnabled(windows::Win32::Foundation::VARIANT_BOOL(-1))?;
        }
        Ok(())
    }

    pub(crate) fn run_helper_if_requested() -> Option<i32> {
        let args: Vec<String> = env::args().collect();
        if args.get(1).map(String::as_str) != Some(HELPER_FLAG) {
            return None;
        }
        let action = args.get(2).map(String::as_str);
        let port = args.get(3).and_then(|value| value.parse::<u16>().ok());
        let code = match (action, port, args.len()) {
            (Some(ACTION_ENSURE), Some(port), 4) => helper_ensure(port),
            (Some(ACTION_REMOVE), Some(port), 4) => helper_remove(port),
            _ => EXIT_RULE_UPDATE_FAILED,
        };
        Some(code as i32)
    }

    fn helper_remove(_port: u16) -> u32 {
        let Ok(_com) = ComGuard::initialize() else {
            return EXIT_RULE_UPDATE_FAILED;
        };
        let Ok(policy) = policy() else {
            return EXIT_SERVICE_UNAVAILABLE;
        };
        let Ok(rules) = (unsafe { policy.Rules() }) else {
            return EXIT_RULE_UPDATE_FAILED;
        };
        let name = BSTR::from(RULE_NAME);
        let Ok(rule) = (unsafe { rules.Item(&name) }) else {
            return EXIT_OK;
        };
        let owned = match unsafe { rule.Grouping() } {
            Ok(group) => group.to_string() == RULE_GROUP,
            Err(error) => return map_rule_update_error(error),
        };
        if !owned {
            return EXIT_OK;
        }
        match unsafe { rules.Remove(&name) } {
            Ok(()) => EXIT_OK,
            Err(error) => map_rule_update_error(error),
        }
    }

    fn launch_helper(action: &str, port: u16) -> Result<u32, ShellRuntimeError> {
        let executable = env::current_exe().map_err(|_| firewall_error("无法定位防火墙 helper"))?;
        let executable = wide(executable.as_os_str());
        let parameters = wide(OsStr::new(&format!("{HELPER_FLAG} {action} {port}")));
        let verb = wide(OsStr::new("runas"));
        let mut execute_info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            nShow: 0,
            ..Default::default()
        };
        unsafe { ShellExecuteExW(&mut execute_info) }.map_err(|error| {
            if is_win32_hresult(error.code().0 as u32, ERROR_CANCELLED.0) {
                ShellRuntimeError::new(
                    crate::contract::ERROR_FIREWALL_AUTHORIZATION_REQUIRED,
                    "需要允许 Windows 防火墙访问局域网连接",
                )
            } else {
                firewall_error("无法启动 Windows 防火墙授权")
            }
        })?;
        let process = execute_info.hProcess;
        if process.is_invalid() {
            return Err(firewall_error("Windows 防火墙 helper 未返回进程"));
        }
        unsafe {
            WaitForSingleObject(process, INFINITE);
            let mut exit_code = EXIT_RULE_UPDATE_FAILED;
            let _ = GetExitCodeProcess(process, &mut exit_code);
            let _ = CloseHandle(process);
            Ok(exit_code)
        }
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn firewall_error(message: &str) -> ShellRuntimeError {
        ShellRuntimeError::new(crate::contract::ERROR_FIREWALL_RULE_UPDATE_FAILED, message)
    }

    fn map_policy_error(error: windows::core::Error, message: &str) -> ShellRuntimeError {
        if is_win32_hresult(error.code().0 as u32, ERROR_ACCESS_DENIED.0) {
            ShellRuntimeError::new(
                crate::contract::ERROR_FIREWALL_POLICY_BLOCKED,
                "系统策略阻止读取或修改 Windows 防火墙规则",
            )
        } else {
            firewall_error(message)
        }
    }

    fn map_rule_update_error(error: windows::core::Error) -> u32 {
        if is_win32_hresult(error.code().0 as u32, ERROR_ACCESS_DENIED.0) {
            EXIT_POLICY_BLOCKED
        } else {
            EXIT_RULE_UPDATE_FAILED
        }
    }

    fn is_win32_hresult(value: u32, win32_code: u32) -> bool {
        value == win32_code || value == 0x8007_0000_u32 | win32_code
    }
}

/// 在测试模式或非 Windows 平台不执行系统防火墙修改。
pub(crate) fn ensure(
    port: u16,
    desktop_process: bool,
) -> Result<RuntimeFirewallSnapshot, ShellRuntimeError> {
    #[cfg(windows)]
    {
        if !desktop_process {
            return Ok(RuntimeFirewallSnapshot {
                status: "not-required".to_owned(),
                port: Some(port),
                scope: Some(SCOPE_LOCAL_SUBNET.to_owned()),
            });
        }
        return windows_impl::ensure(port);
    }
    #[cfg(not(windows))]
    {
        let _ = (port, desktop_process);
        Err(ShellRuntimeError::new(
            crate::contract::ERROR_FIREWALL_RULE_UPDATE_FAILED,
            "当前版本仅支持 Windows 自动配置局域网访问",
        ))
    }
}

/// 只读检查当前端口的防火墙状态；不会启动 UAC helper。
pub(crate) fn probe(
    port: u16,
    desktop_process: bool,
) -> Result<RuntimeFirewallSnapshot, ShellRuntimeError> {
    #[cfg(windows)]
    {
        if !desktop_process {
            return Ok(RuntimeFirewallSnapshot {
                status: "not-required".to_owned(),
                port: Some(port),
                scope: Some(SCOPE_LOCAL_SUBNET.to_owned()),
            });
        }
        return windows_impl::probe(port);
    }
    #[cfg(not(windows))]
    {
        let _ = (port, desktop_process);
        Err(ShellRuntimeError::new(
            crate::contract::ERROR_FIREWALL_RULE_UPDATE_FAILED,
            "当前版本仅支持 Windows 自动配置局域网访问",
        ))
    }
}

/// 显式离开 server-mode 时尽力删除当前自有防火墙规则。
pub(crate) fn remove(port: u16, desktop_process: bool) -> Result<(), ShellRuntimeError> {
    #[cfg(windows)]
    {
        if !desktop_process {
            return Ok(());
        }
        return windows_impl::remove(port);
    }
    #[cfg(not(windows))]
    {
        let _ = (port, desktop_process);
        Ok(())
    }
}

/// 正式 Desktop 进程启动前检查是否为受限 helper 分支。
pub fn run_helper_if_requested() -> Option<i32> {
    #[cfg(windows)]
    {
        return windows_impl::run_helper_if_requested();
    }
    #[cfg(not(windows))]
    None
}
