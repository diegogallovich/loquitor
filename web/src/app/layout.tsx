import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  metadataBase: new URL("https://loquitor.reachdiego.com"),
  title: "Loquitor — Let your agents think out loud",
  description:
    "Open source Rust CLI daemon that speaks your AI coding agent's thoughts aloud via TTS.",
  openGraph: {
    title: "Loquitor — Let your agents think out loud",
    description:
      "Open source Rust CLI daemon that speaks your AI coding agent's thoughts aloud via TTS.",
    url: "https://loquitor.reachdiego.com",
    siteName: "Loquitor",
    type: "website",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "Loquitor — Let your agents think out loud",
    description:
      "Open source Rust CLI daemon that speaks your AI coding agent's thoughts aloud via TTS.",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
