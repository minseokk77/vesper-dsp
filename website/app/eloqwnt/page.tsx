"use client";

import styles from "./page.module.css";
import Link from "next/link";
import { useEffect } from "react";

export default function EloqwntConcept() {
  useEffect(() => {
    // Inject DM Sans for the editorial look
    const link = document.createElement("link");
    link.href = "https://fonts.googleapis.com/css2?family=DM+Sans:opsz,wght@9..40,300;9..40,400;9..40,500;9..40,600&display=swap";
    link.rel = "stylesheet";
    document.head.appendChild(link);
    return () => { document.head.removeChild(link); };
  }, []);

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <div>VESPER.</div>
        <div>Strategy-led Audio & Autonomous Coding</div>
      </header>

      <section className={styles.hero}>
        <p>The perception gap closes here.</p>
        <h1>Engineering systems that disappear in operation, but are obvious when you listen.</h1>
        <Link href="/" className={styles.cta}>Explore Main Site</Link>
      </section>

      <section className={styles.statsGrid}>
        <div className={styles.statBox}>
          <div className={styles.statValue}>Stage 5</div>
          <div className={styles.statLabel}>Autonomous Engine</div>
        </div>
        <div className={styles.statBox}>
          <div className={styles.statValue}>Zero</div>
          <div className={styles.statLabel}>Latency Overhead</div>
        </div>
        <div className={styles.statBox}>
          <div className={styles.statValue}>100%</div>
          <div className={styles.statLabel}>Local Execution</div>
        </div>
        <div className={styles.statBox}>
          <div className={styles.statValue}>Infinite</div>
          <div className={styles.statLabel}>Possibilities</div>
        </div>
      </section>

      <nav className={styles.navLinks}>
        <Link href="/">← Main Website</Link>
        <Link href="/dribbble">View Dribbble Concept →</Link>
      </nav>
    </div>
  );
}
