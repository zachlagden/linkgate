import { invoke } from "@tauri-apps/api/core";

export type Signal = "insecure" | "credentials" | "punycode" | "ip" | "port";

export interface LinkView {
  href: string;
  scheme: string;
  separator: string;
  userinfo: string | null;
  subdomain: string | null;
  domain: string | null;
  port: number | null;
  path: string;
  query: string | null;
  fragment: string | null;
  unicodeHost: string | null;
  openable: boolean;
  signals: Signal[];
}

export interface ListHit {
  list: string;
  matched: string;
}

export interface BrowserView {
  id: string;
  name: string;
  path: string;
  icon: string | null;
  isDefault: boolean;
  hidden: boolean;
}

export interface ListStatus {
  name: string;
  count: number;
  updatedAt: number;
}

export interface ListsStatus {
  checkedAt: number;
  lastError: string | null;
  updating: boolean;
  lists: ListStatus[];
}

export interface InitialState {
  link: LinkView | null;
  raw: string | null;
  hits: ListHit[];
  browsers: BrowserView[];
  lists: ListsStatus;
  version: string;
}

export const api = {
  initialState: (): Promise<InitialState> => invoke("initial_state"),
  present: (height: number): Promise<void> => invoke("present", { height }),
  resize: (height: number): Promise<void> => invoke("resize", { height }),
  openIn: (id: string): Promise<void> => invoke("open_in", { id }),
  copyLink: (): Promise<void> => invoke("copy_link"),
  dismiss: (): Promise<void> => invoke("dismiss"),
  setBrowserHidden: (id: string, hidden: boolean): Promise<BrowserView[]> =>
    invoke("set_browser_hidden", { id, hidden }),
  updateLists: (): Promise<ListsStatus> => invoke("update_lists"),
  listsStatus: (): Promise<ListsStatus> => invoke("lists_status"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong.";
}
