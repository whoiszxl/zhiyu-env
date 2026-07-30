const ALLOWED_TAGS = new Set([
  "a",
  "b",
  "blockquote",
  "br",
  "code",
  "del",
  "div",
  "em",
  "figcaption",
  "figure",
  "h1",
  "h2",
  "h3",
  "h4",
  "hr",
  "i",
  "img",
  "li",
  "ol",
  "p",
  "pre",
  "s",
  "span",
  "strong",
  "table",
  "tbody",
  "td",
  "th",
  "thead",
  "tr",
  "u",
  "ul",
]);

const DROP_WITH_CONTENT = new Set([
  "base",
  "button",
  "embed",
  "form",
  "iframe",
  "input",
  "link",
  "meta",
  "object",
  "script",
  "select",
  "style",
  "svg",
  "textarea",
]);

function safeRemoteUrl(value: string | null, baseUrl: string | null): string | null {
  if (!value) return null;
  try {
    const url = new URL(value, baseUrl || undefined);
    return url.protocol === "http:" || url.protocol === "https:"
      ? url.toString()
      : null;
  } catch {
    return null;
  }
}

function sanitizeElement(element: Element, baseUrl: string | null) {
  for (const child of Array.from(element.children)) {
    sanitizeElement(child, baseUrl);
  }

  const tag = element.tagName.toLowerCase();
  if (DROP_WITH_CONTENT.has(tag)) {
    element.remove();
    return;
  }
  if (!ALLOWED_TAGS.has(tag)) {
    element.replaceWith(...Array.from(element.childNodes));
    return;
  }

  const href = tag === "a" ? safeRemoteUrl(element.getAttribute("href"), baseUrl) : null;
  const src = tag === "img" ? safeRemoteUrl(element.getAttribute("src"), baseUrl) : null;
  const title = element.getAttribute("title");
  const alt = element.getAttribute("alt");
  const width = element.getAttribute("width");
  const height = element.getAttribute("height");
  for (const attribute of Array.from(element.attributes)) {
    element.removeAttribute(attribute.name);
  }

  if (tag === "a") {
    if (!href) {
      element.replaceWith(...Array.from(element.childNodes));
      return;
    }
    element.setAttribute("href", href);
    element.setAttribute("target", "_blank");
    element.setAttribute("rel", "noopener noreferrer");
    if (title) element.setAttribute("title", title);
  }
  if (tag === "img") {
    if (!src) {
      element.remove();
      return;
    }
    element.setAttribute("src", src);
    element.setAttribute("loading", "lazy");
    element.setAttribute("decoding", "async");
    element.setAttribute("referrerpolicy", "no-referrer");
    if (alt) element.setAttribute("alt", alt);
    if (title) element.setAttribute("title", title);
    if (width && /^\d{1,4}$/.test(width)) element.setAttribute("width", width);
    if (height && /^\d{1,4}$/.test(height)) element.setAttribute("height", height);
  }
}

export function renderSafeRssHtml(source: string, baseUrl: string | null): string {
  if (!source.trim()) return "";
  const document = new DOMParser().parseFromString(source, "text/html");
  for (const child of Array.from(document.body.children)) {
    sanitizeElement(child, baseUrl);
  }
  return document.body.innerHTML;
}
