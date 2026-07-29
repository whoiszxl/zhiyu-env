import type { DocChapter } from "../docTypes";
import { buildBasicChapters } from "./zh-CN-basics";
import { buildClientChapters } from "./zh-CN-clients";

export function buildRedisDocs(port: number): DocChapter[] {
  return [...buildBasicChapters(port), ...buildClientChapters(port)];
}
