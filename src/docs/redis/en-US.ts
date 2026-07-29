import type { DocChapter } from "../docTypes";
import { buildBasicChapters } from "./en-US-basics";
import { buildClientChapters } from "./en-US-clients";

export function buildRedisDocs(port: number): DocChapter[] {
  return [...buildBasicChapters(port), ...buildClientChapters(port)];
}
