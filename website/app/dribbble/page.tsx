"use client";

import styles from "./page.module.css";
import Link from "next/link";

export default function DribbbleConcept() {
  return (
    <div className={styles.container}>
      <div className={styles.backgroundGlows}>
        <div className={styles.glow1}></div>
        <div className={styles.glow2}></div>
      </div>

      <div className={styles.content}>
        <h1 className={styles.title}>VESPER</h1>
        <p className={styles.subtitle}>
          The future of autonomous coding and audio fidelity, encapsulated in a liquid glass dashboard.
        </p>

        <div className={styles.dashboard}>
          <div className={`${styles.card} ${styles.glowCard}`}>
            <div className={styles.cardIcon}>✦</div>
            <h3>Stage 1: Analyze</h3>
            <p>Scanning contextual memory and project requirements in real-time with zero latency.</p>
          </div>
          <div className={`${styles.card} ${styles.glowCard}`}>
            <div className={styles.cardIcon}>◉</div>
            <h3>Stage 2: Risk Scan</h3>
            <p>Deep Ollama security analysis protecting your environment before execution.</p>
          </div>
          <div className={`${styles.card} ${styles.glowCard}`}>
            <div className={styles.cardIcon}>⌁</div>
            <h3>Stage 3: Execute</h3>
            <p>Seamless orchestration of agents and dynamic Liquid Glass token generation.</p>
          </div>
        </div>

        <div style={{ marginTop: "4rem" }}>
          <Link href="/" style={{ color: "#34d399", textDecoration: "none", borderBottom: "1px solid #34d399", paddingBottom: "2px" }}>
            ← Back to Main
          </Link>
          <Link href="/eloqwnt" style={{ color: "#60a5fa", textDecoration: "none", borderBottom: "1px solid #60a5fa", paddingBottom: "2px", marginLeft: "24px" }}>
            Compare with Eloqwnt Concept →
          </Link>
        </div>
      </div>
    </div>
  );
}
