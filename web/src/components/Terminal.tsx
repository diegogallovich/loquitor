import styles from "./Terminal.module.css";

export default function Terminal({ children }: { children: React.ReactNode }) {
  return (
    <div className={styles.wrapper}>
      <div className={styles.terminal}>
        <div className={styles.titleBar}>
          <div className={styles.dots}>
            <div className={`${styles.dot} ${styles.dotRed}`} />
            <div className={`${styles.dot} ${styles.dotYellow}`} />
            <div className={`${styles.dot} ${styles.dotGreen}`} />
          </div>
          <div className={styles.title}>
            diego@Diegos-MBP — zsh — 80×24
          </div>
        </div>
        <div className={styles.content}>{children}</div>
      </div>
    </div>
  );
}
