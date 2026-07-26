import type { DocChapter } from "./docTypes";
import { buildBasicChapters } from "./redisDocsBasics";
import { buildClientChapters } from "./redisDocsClients";

export function buildRedisDocs(port: number): DocChapter[] {
  return [...buildBasicChapters(port), ...buildClientChapters(port)];
}
