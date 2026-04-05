import { invoke } from "@tauri-apps/api/core";

let urlInput: HTMLInputElement | null;
let tokenInput: HTMLInputElement | null;

async function saveSettings() {
  if (urlInput) {
    await invoke("save_url", { url: urlInput.value });
  }
  if (tokenInput && tokenInput.value.trim()) {
    await invoke("save_token", { token: tokenInput.value.trim() });
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  urlInput = document.querySelector("#url-input");
  tokenInput = document.querySelector("#token-input");

  const currentUrl: string | null = await invoke("get_url");
  if (urlInput && currentUrl) {
    urlInput.value = currentUrl;
  }

  const currentToken: string | null = await invoke("get_token");
  if (tokenInput && currentToken) {
    tokenInput.value = currentToken;
  }

  document.querySelector("#save-form")?.addEventListener("submit", e => {
    e.preventDefault();
    saveSettings();
  });
});
