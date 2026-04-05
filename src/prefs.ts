import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface Prefs {
  device_name: string | null;
  auto_update: boolean;
  notifications_enabled: boolean;
}

window.addEventListener("DOMContentLoaded", async () => {
  const deviceNameInput = document.querySelector<HTMLInputElement>("#device-name")!;
  const autoUpdateCheckbox = document.querySelector<HTMLInputElement>("#auto-update")!;
  const notifsCheckbox = document.querySelector<HTMLInputElement>("#notifs-enabled")!;
  const checkUpdateBtn = document.querySelector<HTMLButtonElement>("#check-update")!;
  const updateStatus = document.querySelector<HTMLParagraphElement>("#update-status")!;
  const saveBtn = document.querySelector<HTMLButtonElement>("#save")!;
  const cancelBtn = document.querySelector<HTMLButtonElement>("#cancel")!;

  // Load current prefs
  const prefs: Prefs = await invoke("get_prefs");
  deviceNameInput.value = prefs.device_name ?? "";
  autoUpdateCheckbox.checked = prefs.auto_update;
  notifsCheckbox.checked = prefs.notifications_enabled;

  // Check for updates now
  checkUpdateBtn.addEventListener("click", async () => {
    checkUpdateBtn.disabled = true;
    checkUpdateBtn.textContent = "Checking…";
    updateStatus.textContent = "";
    try {
      const result: string = await invoke("check_update_now");
      updateStatus.textContent = result;
    } catch (e) {
      updateStatus.textContent = String(e);
    } finally {
      checkUpdateBtn.disabled = false;
      checkUpdateBtn.textContent = "Check for Updates";
    }
  });

  // Save
  saveBtn.addEventListener("click", async () => {
    const deviceName = deviceNameInput.value.trim() || null;
    await invoke("save_prefs", {
      deviceName,
      autoUpdate: autoUpdateCheckbox.checked,
      notificationsEnabled: notifsCheckbox.checked,
    });
  });

  // Cancel
  cancelBtn.addEventListener("click", () => {
    getCurrentWindow().close();
  });
});
