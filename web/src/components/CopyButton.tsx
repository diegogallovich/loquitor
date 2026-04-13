"use client";

import { useState } from "react";
import styles from "./CopyButton.module.css";

export default function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API can fail in insecure contexts or if permissions are denied.
      // Silently no-op — the command is still visible on the page for manual copy.
    }
  };

  return (
    <button
      className={styles.button}
      onClick={handleCopy}
      title="Copy to clipboard"
      aria-label={copied ? "Copied" : "Copy to clipboard"}
    >
      {copied ? "copied!" : "copy"}
    </button>
  );
}
