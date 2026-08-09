// English copy mirroring zh-CN/startup.ts; keys must match the zh baseline exactly.
export const startup = {
  wizardSubtitle: "Complete the WineStock first-run setup.",
  serviceUnavailableTitle: "Cannot reach the WineStock service",
  serviceUnavailableDescription: "The service is currently unavailable. Check runtime settings or retry later.",
  serviceRecovery: "Recovering local service…",
  wizardWelcomeTitle: "Welcome to WineStock",
  wizardModeIntro:
    "First choose how this device is used; you can change this anytime in settings.",
  wizardUsageAriaLabel: "Usage mode",
  recommended: "Recommended",
  modeLocalLabel: "Local only",
  modeLocalDescription:
    "Service and data run on this device, ideal for personal or single-device use.",
  modeServerLabel: "Shared service",
  modeServerDescription: "Run the service on this device and let other devices connect.",
  modeRemoteLabel: "Connect to remote",
  modeRemoteDescription:
    "Do not start a local service; connect to an existing WineStock service.",
  serverTitle: "Connect to server",
  serverIntro:
    "Enter the server address, usually provided by whoever deploys WineStock.",
  serverAddressLabel: "Server address",
  serverAddressHint: "Example: http://192.168.1.10:17890",
  consentTitle: "Preferences",
  consentIntro:
    "These options only affect this device and can be changed anytime later.",
  appearanceTitle: "Appearance",
  telemetryTitle: "Data collection",
  telemetryConsentLabel: "Send anonymous usage data",
  telemetryConsentDescription:
    "Helps developers locate and fix issues; does not include inventory content or account information, and only works while online. Analytics is provided by Microsoft Clarity.",
  telemetryDefaultNote: "Enabled by default; you can turn it off.",
  telemetryPolicyLink: "View Microsoft privacy statement",
  applyingLoading: "Loading…",
  backToEdit: "Back to edit",
  retry: "Retry",
  previous: "Back",
  finish: "Finish",
  next: "Next",
  checkServerAddress: "Check the server address",
  serverAddressInvalid: "Invalid server address. Please check and retry.",
  remoteTestFailed: "Remote connection test failed",
  remoteTestTimeout:
    "Connection timed out. Check the server address and network connection.",
  remoteTestUnreachable:
    "Cannot connect right now. Check the server address and network connection.",
  applyConfigFailedTitle: "Failed to apply runtime configuration",
  remoteTestFailedApply:
    "Remote connection test failed. Go back to fix the server address or check the network.",
  applyConfigFailedRetry:
    "Could not apply the current configuration. Go back to edit and retry.",
  applyConfigFailedLater:
    "Could not apply the current configuration. Please retry later.",
  runtimeMode: "Runtime mode",
  connectingTitle: "Connecting",
  connectingBody: "Checking service status, please wait.",
  reconnect: "Reconnect",
  connectingBusy: "Connecting…",
  localFailedTitle: "Local service error",
  localFailedBody:
    "Automatic recovery of the local service failed. Retry starting it, or check the configuration in runtime settings.",
  restartService: "Restart service",
  startingBusy: "Starting…",
  localStoppedTitle: "Local service stopped",
  localStoppedBody:
    "The local service is currently stopped. It will return to this page after starting.",
  startService: "Start service",
  remoteUnreachableTitle: "Cannot reach the service right now",
  remoteUnreachableBody:
    "Make sure the service is running and the network connection is normal. The app retries automatically and returns to this page when the connection is restored.",
};
