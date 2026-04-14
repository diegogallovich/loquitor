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

        <InfoCard
          title="Loquitor"
          subtitle="Hear when your agents need you"
        />
        <div style={{ color: "var(--term-dim)", paddingLeft: "3ch" }}>
          v0.2.0 · MIT License
        </div>

        <br />

        <div style={{ color: "var(--term-text-dim)" }}>
          Rust CLI daemon that watches your AI coding agent&apos;s
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          terminal output, waits until each turn finishes, and
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          speaks one short summary so you know what just shipped
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          and what it&apos;s waiting for. Smart notifications,
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          not a running monologue.
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Features
        </div>
        <Divider style="single" />
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Idle detection
          per session — fires only when Claude stops
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> One LLM-written
          sentence per turn, prefixed &quot;Regarding {"{session}"}.&quot;
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Multi-provider:
          Anthropic, OpenAI, MiniMax for summaries
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Multi-provider
          TTS: OpenAI, ElevenLabs, MiniMax, macOS Say
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Concurrent
          summaries across lanes, played in turn-end order
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          <span style={{ color: "var(--term-green)" }}>✓</span> Secret scrubber
          before any cloud LLM call (sk-…, ghp_…, JWT, etc.)
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Quick Install
        </div>
        <Divider style="single" />
        <div style={{ color: "var(--term-text-dim)" }}>
          Pre-built binaries for macOS (Intel + Apple Silicon)
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          and Linux (x86_64 + aarch64) on every release:
        </div>
        <div style={{ marginTop: "8px" }}>
          <span style={{ color: "var(--term-green)" }}>→</span>{" "}
          <a
            href="https://github.com/diegogallovich/loquitor/releases/latest"
            target="_blank"
            rel="noopener noreferrer"
          >
            github.com/diegogallovich/loquitor/releases/latest
          </a>
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "12px" }}>
          Or build from source:
        </div>
        <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }}>
          <span>
            <span style={{ color: "var(--term-green)" }}>$</span> git clone
            https://github.com/diegogallovich/loquitor
          </span>
          <CopyButton text="git clone https://github.com/diegogallovich/loquitor" />
        </div>
        <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }}>
          <span>
            <span style={{ color: "var(--term-green)" }}>$</span> cd loquitor
            &amp;&amp; cargo install --path .
          </span>
          <CopyButton text="cd loquitor && cargo install --path ." />
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "8px" }}>
          {"  "}cargo install loquitor and brew tap install
        </div>
        <div style={{ color: "var(--term-muted)" }}>
          {"  "}arrive in a near-future patch.
        </div>

        <br />

        <div style={{ color: "var(--term-white)", fontWeight: "bold" }}>
          Getting Started
        </div>
        <Divider style="single" />
        <div>
          <span style={{ color: "var(--term-green)" }}>$</span> loquitor init
          <span style={{ color: "var(--term-muted)" }}>
            {"    "}# Pick TTS + summary LLM, models, voice
          </span>
        </div>
        <div>
          <span style={{ color: "var(--term-green)" }}>$</span> loquitor enable
          <span style={{ color: "var(--term-muted)" }}>
            {"  "}# Install shell hook + start daemon
          </span>
        </div>
        <div style={{ color: "var(--term-muted)", marginTop: "4px" }}>
          Then open a new terminal tab and run claude. Loquitor
        </div>
        <div style={{ color: "var(--term-muted)" }}>
          detects each session, waits for Claude to finish a turn,
        </div>
        <div style={{ color: "var(--term-muted)" }}>
          and announces what happened.
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
          capturing output to a per-session log.
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          The daemon tails the log, detects when Claude&apos;s
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          input prompt stabilises (turn ended), scrubs secrets
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          from the buffer, sends it to your chosen summary LLM,
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          prepends &quot;Regarding {"{session}"}.&quot;, and plays
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          the result through your TTS provider — one sentence,
        </div>
        <div style={{ color: "var(--term-text-dim)" }}>
          one notification, queued in turn-end order across lanes.
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
          {"  "}Issues · Pull Requests · Discussions
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
