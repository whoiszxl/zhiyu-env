function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function safeUrl(value: string): string | null {
  const url = value.trim();
  return /^(https?:\/\/|mailto:)/i.test(url) ? escapeHtml(url) : null;
}

function inlineMarkdown(source: string): string {
  const codeTokens: string[] = [];
  let value = escapeHtml(source).replace(/`([^`\n]+)`/g, (_, code: string) => {
    const token = `\u0000CODE${codeTokens.length}\u0000`;
    codeTokens.push(`<code>${code}</code>`);
    return token;
  });
  value = value
    .replace(
      /\[([^\]]+)\]\(([^)\s]+)\)/g,
      (_, label: string, url: string) => {
        const href = safeUrl(url);
        return href
          ? `<a href="${href}" target="_blank" rel="noreferrer">${label}</a>`
          : label;
      },
    )
    .replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>")
    .replace(/__([^_\n]+)__/g, "<strong>$1</strong>")
    .replace(/~~([^~\n]+)~~/g, "<del>$1</del>")
    .replace(/(^|[\s(])\*([^*\n]+)\*(?=$|[\s).,!?])/g, "$1<em>$2</em>");
  return value.replace(/\u0000CODE(\d+)\u0000/g, (_, index: string) => {
    return codeTokens[Number(index)] ?? "";
  });
}

export function renderMarkdown(source: string): string {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const output: string[] = [];
  let paragraph: string[] = [];
  let list: "ul" | "ol" | null = null;
  let inCode = false;
  let codeLanguage = "";
  let code: string[] = [];

  const flushParagraph = () => {
    if (!paragraph.length) return;
    output.push(`<p>${paragraph.map(inlineMarkdown).join("<br>")}</p>`);
    paragraph = [];
  };
  const closeList = () => {
    if (!list) return;
    output.push(`</${list}>`);
    list = null;
  };
  const flushCode = () => {
    const language = codeLanguage.replace(/[^a-z0-9_+-]/gi, "").slice(0, 24);
    output.push(
      `<pre data-language="${escapeHtml(language)}"><code>${escapeHtml(code.join("\n"))}</code></pre>`,
    );
    code = [];
    codeLanguage = "";
  };

  for (const line of lines) {
    const fence = line.match(/^```(.*)$/);
    if (fence) {
      if (inCode) flushCode();
      else {
        flushParagraph();
        closeList();
        codeLanguage = fence[1].trim();
      }
      inCode = !inCode;
      continue;
    }
    if (inCode) {
      code.push(line);
      continue;
    }
    if (!line.trim()) {
      flushParagraph();
      closeList();
      continue;
    }
    const heading = line.match(/^(#{1,4})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1].length + 1;
      output.push(`<h${level}>${inlineMarkdown(heading[2])}</h${level}>`);
      continue;
    }
    if (/^\s*([-*_])(?:\s*\1){2,}\s*$/.test(line)) {
      flushParagraph();
      closeList();
      output.push("<hr>");
      continue;
    }
    const unordered = line.match(/^\s*[-*+]\s+(.+)$/);
    const ordered = line.match(/^\s*\d+[.)]\s+(.+)$/);
    if (unordered || ordered) {
      flushParagraph();
      const target = unordered ? "ul" : "ol";
      if (list !== target) {
        closeList();
        list = target;
        output.push(`<${target}>`);
      }
      output.push(`<li>${inlineMarkdown((unordered ?? ordered)![1])}</li>`);
      continue;
    }
    const quote = line.match(/^\s*>\s?(.*)$/);
    if (quote) {
      flushParagraph();
      closeList();
      output.push(`<blockquote>${inlineMarkdown(quote[1])}</blockquote>`);
      continue;
    }
    closeList();
    paragraph.push(line);
  }
  if (inCode) flushCode();
  flushParagraph();
  closeList();
  return output.join("");
}
