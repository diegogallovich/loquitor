export default function PromptLine({ command }: { command?: string }) {
  return (
    <div>
      <span style={{ color: "var(--term-green)" }}>diego@Diegos-MBP</span>{" "}
      <span style={{ color: "var(--term-blue)" }}>~</span>{" "}
      <span style={{ color: "var(--term-text)" }}>%</span>
      {command && (
        <span style={{ color: "var(--term-text)" }}> {command}</span>
      )}
    </div>
  );
}
