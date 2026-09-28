const LIST_LABELS: Record<string, string> = {
  malicious: "malicious",
  suspicious: "suspicious",
  tracking: "tracking",
};

export function listLabel(name: string): string {
  return LIST_LABELS[name] ?? name;
}

export function joinWords(words: string[]): string {
  if (words.length <= 1) return words.join("");
  return `${words.slice(0, -1).join(", ")} and ${words[words.length - 1]}`;
}

export function capitalise(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1);
}

export function relativeTime(epochSeconds: number): string {
  if (!epochSeconds) return "never";
  const seconds = Math.max(0, Date.now() / 1000 - epochSeconds);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "yesterday" : `${days} days ago`;
}

export function formatCount(count: number): string {
  return new Intl.NumberFormat("en-GB").format(count);
}
