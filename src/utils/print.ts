// Impresión / descarga / share sin dependencias.
//
// Desktop: imprime el HTML en un iframe oculto (WebView2/WebKitGTK).
// Mobile: si el WebView no imprime, se ofrece compartir el texto con el
// sharesheet del sistema (mismo criterio mobile-aware que `revealItemInDir`).

import { invoke } from "@tauri-apps/api/core";

export function printHtml(html: string): boolean {
  if (typeof document === "undefined") return false;

  const iframe = document.createElement("iframe");
  iframe.setAttribute("aria-hidden", "true");
  iframe.style.position = "fixed";
  iframe.style.right = "0";
  iframe.style.bottom = "0";
  iframe.style.width = "0";
  iframe.style.height = "0";
  iframe.style.border = "0";
  document.body.appendChild(iframe);

  const doc = iframe.contentDocument;
  const win = iframe.contentWindow;
  if (!doc || !win) {
    iframe.remove();
    return false;
  }

  doc.open();
  doc.write(html);
  doc.close();

  try {
    win.focus();
    win.print();
  } catch {
    iframe.remove();
    return false;
  }

  window.setTimeout(() => iframe.remove(), 1500);
  return true;
}

// El `<a download>` con blob no funciona de forma consistente en los webviews
// de Tauri. El camino principal es el diálogo nativo + escritura en Rust; si
// no hay runtime (dev en navegador) o es mobile (donde `dialog.save` devuelve
// un `content://` que Rust no puede escribir), queda el sharesheet.
export async function downloadText(
  filename: string,
  content: string,
  mime = "text/markdown;charset=utf-8",
): Promise<boolean> {
  try {
    const path = await invoke<string | null>("save_text_file", {
      defaultFilename: filename,
      content,
    });
    return path !== null;
  } catch {
    // Sin runtime de Tauri (dev en navegador) o diálogo no disponible:
    // en mobile el sharesheet es el fallback correcto; en web, el anchor.
    if (await shareText(filename, content)) return true;
    return downloadWithAnchor(filename, content, mime);
  }
}

function downloadWithAnchor(
  filename: string,
  content: string,
  mime: string,
): boolean {
  if (typeof document === "undefined") return false;
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
  return true;
}

type ShareNavigator = Navigator & {
  share?: (data: { title?: string; text?: string }) => Promise<void>;
};

export function canShare(): boolean {
  return typeof navigator !== "undefined" && typeof (navigator as ShareNavigator).share === "function";
}

export async function shareText(title: string, text: string): Promise<boolean> {
  const nav = navigator as ShareNavigator;
  if (typeof nav.share !== "function") return false;
  try {
    await nav.share({ title, text });
    return true;
  } catch {
    return false;
  }
}
