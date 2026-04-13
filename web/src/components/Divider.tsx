import { BOX_WIDTH } from "./terminal-layout";

export default function Divider({ style = "single" }: { style?: "single" | "double" }) {
  const char = style === "double" ? "═" : "─";
  return (
    <div style={{ color: "var(--term-dim)" }}>{char.repeat(BOX_WIDTH)}</div>
  );
}
