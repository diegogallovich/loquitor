import Terminal from "@/components/Terminal";
import PromptLine from "@/components/PromptLine";
import InfoCard from "@/components/InfoCard";
import Divider from "@/components/Divider";
import CopyButton from "@/components/CopyButton";

// Revalidate daily for any time-sensitive content in the future.
export const revalidate = 86400;

export default function Home() {
  return (
    <main>
      <Terminal>
        <PromptLine command="cd projects/loquitor" />

        <br />

        <InfoCard title="Loquitor" subtitle="Let your agents think out loud" />
        <div style={{ color: "var(--term-dim)", paddingLeft: "3ch" }}>
          v0.1.0 · MIT License
        </div>

        <br />

        <div style={{ color: "var(--term-text-dim)" }}>
          Rust CLI daemon that watches your AI coding agent&apos;s
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          terminal output and speaks its narrative thoughts
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          aloud via TTS. Two commands to set up, then forget it.
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Features
        </div>
        <Divider style="single" />
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Auto-detects
          Claude Code sessions via shell hook
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Multiple voices
          for parallel agents
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> OpenAI,
          ElevenLabs, MiniMax, macOS Say
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Smart filtering
          — speaks thoughts, skips code and tool calls
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Stale-drop queue
          keeps narration current during bursts
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Quick Install
        </div>
        <Divider style="single" />
        <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }}>
          <span>
            <span style={{ color: "var(--term-green)" }}>$</span> cargo install
            loquitor
          </span>
          <CopyButton text="cargo install loquitor" />
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "4px" }}>or</div>
        <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }}>
          <span>
            <span style={{ color: "var(--term-green)" }}>$</span> brew install
            diegogallovich/tap/loquitor
          </span>
          <CopyButton text="brew install diegogallovich/tap/loquitor" />
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "8px" }}>
          Or download a pre-built binary from{" "}
          <a
            href="https://github.com/diegogallovich/loquitor/releases"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub Releases
          </a>
          .
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Getting Started
        </div>
        <Divider style="single" />
        <div>
          <span style={{ color: "var(--term-green)" }}>$</span> loquitor init
          <span style={{ color: "var(--term-muted)" }}>
            {"  "}# One-time setup: TTS provider, voice, test
          </span>
        </div>
        <div>
          <span style={{ color: "var(--term-green)" }}>$</span> loquitor enable
          <span style={{ color: "var(--term-muted)" }}>
            {" "}# Install shell hook + start daemon
          </span>
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "4px" }}>
          Then open a new terminal tab and run claude. Loquitor
        </div>
        <div style={{ color: "var(--term-muted)" }}>
          detects the session and starts narrating automatically.
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          How It Works
        </div>
        <Divider style="single" />
        <div style={{ color: "var(--term-text-dim)" }}>
          A shell hook wraps the{" "}
          <span style={{ color: "var(--term-green)" }}>claude</span> command
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          with{" "}
          <span style={{ color: "var(--term-green)" }}>script -q</span>,
          capturing output to a log file.
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          The daemon tails the log, filters for narrative
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          thought lines (skipping tool calls, code, file
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          paths, diagrams), synthesizes via your chosen TTS
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          provider, and plays through a single global queue.
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Contribute
        </div>
        <Divider style="single" />
        <div>
          <span style={{ color: "var(--term-green)" }}>→</span>{" "}
          <a
            href="https://github.com/diegogallovich/loquitor"
            target="_blank"
            rel="noopener noreferrer"
          >
            github.com/diegogallovich/loquitor
          </a>
        </div>
        <div style={{ color: "var(--term-muted)" }}>
          {"   "}Issues · Pull Requests · Discussions
        </div>

        <br />

        <Divider style="double" />

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Tip the Creator
        </div>
        <Divider style="single" />
        <div style={{ color: "var(--term-text-dim)" }}>
          Loquitor is free and open source. If it saves you
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          time, consider tipping.
        </div>
        <div style={{ color: "var(--term-text-dim)", marginTop: "8px" }}>
          Easiest path —{" "}
          <a
            href="https://t.me/diegogallovich"
            target="_blank"
            rel="noopener noreferrer"
          >
            @diegogallovich on Telegram
          </a>
          .
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          Settle in whatever currency works for both of us.
        </div>

        <div style={{ color: "var(--term-muted)", marginTop: "12px" }}>
          On-chain (one wallet per chain, accepts native + stables):
        </div>
        <div style={{ marginTop: "4px" }}>
          <TipRow
            chain="Ethereum"
            accepts="ETH, USDC, USDT"
            address="0xeA284b3EAd48388174d7A67c63DC1a3107FbEA16"
          />
          <TipRow
            chain="Solana"
            accepts="SOL, USDC, USDT"
            address="BjykpVzwfBYqwN6oNieCKdTux7Derm9n1dqJtGoHSeQv"
          />
          <TipRow
            chain="TON"
            accepts="TON, USDT"
            address="UQA6_sZRQkkHspUssT7ruDwhDba3GuGR5qxVPtk2rDZlrLnc"
          />
          <TipRow
            chain="Tron"
            accepts="TRX, USDT"
            address="TWLftLqDRHJNXNv3UGF5vTALE2iXxhkyvF"
          />
          <TipRow
            chain="Bitcoin"
            accepts="BTC"
            address="bc1qrsnavtmh97rqvvgusva3c0ytkrvammuhccxpdv"
          />
        </div>
      </Terminal>
    </main>
  );
}

function TipRow({
  chain,
  accepts,
  address,
}: {
  chain: string;
  accepts: string;
  address: string;
}) {
  return (
    <div style={{ marginTop: "6px" }}>
      <div>
        <span style={{ color: "var(--term-blue)" }}>{chain}</span>{" "}
        <span style={{ color: "var(--term-muted)" }}>({accepts})</span>
      </div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          flexWrap: "wrap",
          paddingLeft: "2ch",
        }}
      >
        <code style={{ color: "var(--term-text)", wordBreak: "break-all" }}>
          {address}
        </code>
        <CopyButton text={address} />
      </div>
    </div>
  );
}
