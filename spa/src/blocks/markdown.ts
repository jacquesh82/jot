export type SplitType = "text" | "heading" | "todo" | "quote" | "code" | "divider";
export interface SplitBlock { block_type: SplitType; content: string; indent: number; }

export function splitMarkdown(md: string): SplitBlock[] {
  const out: SplitBlock[] = [];
  let para = "";
  let paraIndent = 0;
  let inCode = false, codeBuf = "", codeIndent = 0;

  const indentOf = (l: string): [number, string] => {
    let spaces = 0, consumed = 0;
    for (const c of l) {
      if (c === " ") { spaces++; consumed++; }
      else if (c === "\t") { spaces += 2; consumed++; }
      else break;
    }
    return [Math.floor(spaces / 2), l.slice(consumed)];
  };
  const flush = () => {
    const t = para.replace(/[\s]+$/g, "");
    if (t) out.push({ block_type: "text", content: t, indent: paraIndent });
    para = "";
  };

  for (const raw of md.split(/\r?\n/)) {
    if (inCode) {
      if (raw.trimStart().startsWith("```")) {
        out.push({ block_type: "code", content: codeBuf.replace(/[\s]+$/g, ""), indent: codeIndent });
        codeBuf = ""; inCode = false;
      } else { codeBuf += raw + "\n"; }
      continue;
    }
    const [indent, rest] = indentOf(raw);
    if (rest.startsWith("```")) { flush(); inCode = true; codeIndent = indent; continue; }
    if (rest.trim() === "") { flush(); continue; }
    if (rest.trim() === "---") { flush(); out.push({ block_type: "divider", content: "", indent }); continue; }
    const h = /^(#{1,6}) (.*)$/.exec(rest);
    if (h) { flush(); out.push({ block_type: "heading", content: rest, indent }); continue; }
    if (/^- \[[ xX]\] /.test(rest)) { flush(); out.push({ block_type: "todo", content: rest, indent }); continue; }
    if (rest.startsWith("> ")) { flush(); out.push({ block_type: "quote", content: rest.slice(2), indent }); continue; }
    if (!para) paraIndent = indent;
    if (para) para += "\n";
    para += rest;
  }
  flush();
  if (inCode && codeBuf) out.push({ block_type: "code", content: codeBuf.replace(/[\s]+$/g, ""), indent: codeIndent });
  return out;
}
