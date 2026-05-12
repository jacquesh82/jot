import type { BlockDto } from "../api";

export interface BlockNode extends BlockDto {
  children: BlockNode[];
  plaintext: string;
}

export function buildTree(blocks: BlockDto[]): BlockNode[] {
  const byId = new Map<string, BlockNode>();
  for (const b of blocks) byId.set(b.id, { ...b, children: [], plaintext: "" });
  const roots: BlockNode[] = [];
  for (const b of blocks) {
    const node = byId.get(b.id)!;
    if (b.parent_block_id && byId.has(b.parent_block_id)) {
      byId.get(b.parent_block_id)!.children.push(node);
    } else {
      roots.push(node);
    }
  }
  const cmp = (a: BlockNode, b: BlockNode) => a.position - b.position;
  const sortRec = (ns: BlockNode[]) => { ns.sort(cmp); ns.forEach(n => sortRec(n.children)); };
  sortRec(roots);
  return roots;
}

export function flatten(roots: BlockNode[]): BlockNode[] {
  const out: BlockNode[] = [];
  const walk = (ns: BlockNode[]) => ns.forEach(n => { out.push(n); if (!n.collapsed) walk(n.children); });
  walk(roots);
  return out;
}

export function positionAfter(siblings: BlockNode[], prev: BlockNode | null): number {
  if (!prev) return (siblings[0]?.position ?? 1) - 1;
  const idx = siblings.findIndex(s => s.id === prev.id);
  const next = siblings[idx + 1];
  if (!next) return prev.position + 1;
  return (prev.position + next.position) / 2;
}
