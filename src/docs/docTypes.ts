export interface DocCodeSample {
  label: string;
  lang: string;
  caption?: string;
  code: string;
}

export type DocBlock =
  | { kind: "text"; value: string }
  | { kind: "list"; items: string[] }
  | { kind: "code"; lang: string; caption?: string; code: string }
  | { kind: "table"; head: string[]; rows: string[][] }
  | { kind: "callout"; tone: "tip" | "warn"; title: string; value: string }
  | { kind: "samples"; samples: DocCodeSample[] };

export interface DocChapter {
  id: string;
  navLabel: string;
  navHint: string;
  title: string;
  intro: string;
  blocks: DocBlock[];
}
