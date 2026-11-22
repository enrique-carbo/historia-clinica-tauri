// Impresión / descarga / share sin dependencias.
//
// Desktop: imprime el HTML en un iframe oculto (WebView2/WebKitGTK).
// Mobile: si el WebView no imprime, se ofrece compartir el texto con el
// sharesheet del sistema (mismo criterio mobile-aware que `revealItemInDir`).

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

export function downloadText(
  filename: string,
  content: string,
  mime = "text/markdown;charset=utf-8",
): void {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
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
