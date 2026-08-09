// English copy mirroring zh-CN/runtime.ts; keys must match the zh baseline exactly.
export const runtime = {
  modeSelfHosted: "Local mode",
  modeServerMode: "Server mode",
  modeClientOnly: "Remote only",
  modeConnectToRemote: "Connect to remote service",
  listenAddress: "Listen address",
  port: "Port",
  remoteBaseUrl: "Remote service address",
  applyAndRestart: "Apply and restart service",
  runtimeMode: "Runtime mode",
  settingsDescriptionFinished:
    "Choose how WineStock runs on this device, or connect to an existing service.",
  settingsDescriptionFirstTime:
    "First choose how this device runs, then save to continue.",
  serviceStatusAriaLabel: "Current service status",
  checking: "Checking…",
  retry: "Retry",
  localModeLabel: "Local only",
  localModeDescription:
    "Service and data run on this device, ideal for personal or single-device use.",
  remoteModeLabel: "Connect to remote",
  remoteModeDescription:
    "Do not start a local service; connect to an existing WineStock service.",
  serverModeLabel: "Shared service",
  serverModeDescription: "Run the service on this device and let other devices connect.",
  webModeNote:
    "A browser cannot start a local service; only connecting to a remote is supported.",
  webConnectRemoteHint:
    "A browser cannot start a local service. Please connect to a remote.",
  saveToConfirmMode: "Save to confirm the runtime mode",
  unsavedChanges: "Unsaved changes",
  serverAddress: "Server address",
  serverAddressHint: "Example: https://server.example.com:17890",
  insecureHttpWarning:
    "This address does not use HTTPS. Only use it on a trusted network.",
  localAutoStartNote:
    "When the app opens, the local service starts automatically and picks an available port.",
  advancedSettings: "Advanced settings",
  servicePort: "Service port",
  portHint: "Usually no change is needed.",
  listenAddressHintDefault: "The default works for most LAN environments.",
  listenAddressHintLocal: "Local mode is fixed to 127.0.0.1.",
  firewallProviderWindows: "Windows Firewall",
  firewallProviderSystem: "System firewall",
  firewallReady: "The current port is allowed for LAN access.",
  firewallRequiresElevation:
    "{provider} authorization is not finished yet; other devices may not be able to connect.",
  firewallBlockedByPolicy:
    "System policy blocks configuring {provider}; other devices may not be able to connect.",
  firewallProfileUnsupported:
    "The current network is public; the LAN port was not opened automatically.",
  firewallDisabled:
    "{provider} is not running, so LAN access status cannot be confirmed.",
  firewallCleanupPending:
    "Old Windows Firewall rules are not fully cleaned up yet; LAN access may still be open.",
  firewallError:
    "{provider} status cannot be confirmed. Retry or check system settings.",
  firewallNotRequired:
    "This platform does not need automatic firewall configuration.",
  firewallCleanupTitle: "Firewall rule cleanup incomplete",
  firewallConfigIncompleteTitle: "{provider} configuration incomplete",
  firewallCleanupDescription:
    "The runtime mode has been switched, but the old LAN allow rules have not been removed yet.",
  firewallConfigIncompleteDescription:
    "LAN devices may not be able to connect to the current service.",
  firewallCleanupDetail:
    "You can keep using the current runtime mode, or confirm {provider} system permission again and retry the cleanup.",
  firewallConfigIncompleteDetail:
    "You can keep using the current service, or confirm {provider} system permission again and retry the configuration.",
  firewallRetryCleanup: "Retry cleanup",
  firewallRetryAuthorize: "Retry authorization",
  firewallRetryButton: "Retry firewall setup",
  cancel: "Cancel",
  confirm: "Confirm",
  testingConnection: "Testing connection…",
  saving: "Saving…",
  testAndSave: "Test and save",
  saveSettings: "Save settings",
  enableLanTitle: "Enable shared service?",
  switchModeTitle: "Switch runtime mode?",
  confirmationPortChange:
    "After saving, the app will use the new service port.",
  confirmationAddressChange:
    "After saving, the app will use the new service address.",
  confirmationLanEnable:
    "After saving, devices on the same network can connect to WineStock.",
  confirmationSessionCleared:
    "After switching services, the current login state will be cleared and you may need to sign in again.",
  confirmationContinue: "Confirm that you want to save this setting.",
  continueAnyway: "Continue anyway",
  repairing: "Retrying…",
  passwordGateTitle: "Set a password for the current user first",
  passwordGateDescription:
    "Before opening access to other devices, set a real password for the current user; other devices will sign in with the current username.",
  currentUserPassword: "Current user password",
  passwordMinHint: "At least 8 characters.",
  confirmPassword: "Confirm password",
  settingUp: "Setting up…",
  setAndContinue: "Set and continue",
  statusNeedsFirewallAuth: "Firewall authorization needed",
  statusFirewallBlocked: "Firewall rules blocked by system policy",
  statusProfileUnsupported:
    "The current network profile does not support automatic allow",
  statusFirewallDisabled:
    "Firewall is not enabled; LAN protection status unknown",
  statusCleanupPending: "Old firewall rules not cleaned up yet",
  statusFirewallError: "Failed to update firewall rules",
  statusStarting: "Starting",
  statusStopping: "Stopping",
  statusLocalFailed: "Local service failed to start",
  statusNotConnected: "Not connected to a service yet",
  statusAvailable: "Service connection is healthy",
  statusUnavailable: "Cannot reach the service right now",
  statusChecking: "Checking connection",
  runtimeInitFailed: "Failed to initialize the runtime environment",
  checkRuntimeMode: "Check the runtime mode",
  checkInput: "Check the input",
  passwordStatusFailed: "Cannot confirm the current user password status",
  retryLater: "Please retry later.",
  passwordTooShort: "Password needs at least 8 characters",
  passwordMismatch: "The two passwords do not match",
  checkUserAccount: "Check the current user account",
  accountPasswordSet: "Current user password has been set",
  setPasswordFailed: "Failed to set the current user password",
  opFailedRetry: "Please retry.",
  saveFailed: "Failed to save settings",
  saveNotApplied: "Settings were not saved. Check and retry.",
  modeSavedFirewallPending:
    "Runtime mode saved, but the firewall is not finished",
  firewallPendingDetail:
    "You can continue using it, or retry the firewall operation here.",
  modeSaved: "Runtime mode saved",
  remoteTestFailed: "Remote connection test failed",
  remoteTestTimeout:
    "Connection timed out. Check the server address and network connection.",
  remoteTestUnreachable:
    "Cannot connect right now. Check the server address and network connection.",
  firewallCleaned: "Firewall rules cleaned up",
  firewallConfigured: "Firewall setup completed",
  firewallOpFailed: "Firewall operation failed",
  saveModeFirst: "Save the runtime mode first to continue.",
  saveModeFirstDetail: "You can leave this page only after saving.",
  serverModeDisabledWeb:
    "A browser cannot start a local service, so shared service cannot be enabled.",
  serverModeDisabledAndroid:
    "Android only supports local 127.0.0.1 right now, so shared service cannot be enabled.",
  firewallManualConfig:
    "This platform does not support automatic firewall configuration yet. Please configure it manually.",
  lanDialogTitle: "LAN addresses on this device",
  lanDialogDescription:
    "The addresses below belong to this device, for other reachable devices to connect to WineStock.",
  lanCopyLabel: "Connection address",
  copyAddress: "Copy address",
  copyAddressAria: "Copy connection address {url}",
  lanEmpty: "No LAN connection addresses available.",
  lanGuidance:
    "If connecting fails, make sure the device can reach this machine on the network and check the OS firewall.",
  lanInsecureWarning:
    "The list includes plain HTTP addresses; only share them with devices on a trusted network.",
  close: "Close",
};
