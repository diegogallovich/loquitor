import { BOX_INNER_WIDTH, BOX_WIDTH } from "./terminal-layout";

const HORIZONTAL = "─".repeat(BOX_WIDTH - 2);

export default function InfoCard({
  title,
  subtitle,
}: {
  title: string;
  subtitle: string;
}) {
  return (
    <div style={{ color: "var(--term-blue)" }}>
      <div>┌{HORIZONTAL}┐</div>
      <div>
        │{"  "}
        <span style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          {title.padEnd(BOX_INNER_WIDTH)}
        </span>
        │
      </div>
      <div>
        │{"  "}
        <span style={{ color: "var(--term-muted)" }}>
          {subtitle.padEnd(BOX_INNER_WIDTH)}
        </span>
        │
      </div>
      <div>└{HORIZONTAL}┘</div>
    </div>
  );
}
