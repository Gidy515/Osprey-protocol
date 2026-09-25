import Link from "next/link";

import { OspreyLogo } from "@/components/brand/osprey-logo";
import ScrollReveal from "@/components/motion/scroll-reveal";

import styles from "./landing-page.module.css";

export default function LandingPage() {
  return (
    <main className={styles.page}>
      <ScrollReveal />
      <nav className={styles.nav}>
        <Link href="/" className={styles.logoLink}>
          <OspreyLogo />
        </Link>

        <div className={styles.navLinks}>
          <a href="#markets">Markets</a>
          <a href="#risk">Risk Engine</a>
          <a href="#infrastructure">Infrastructure</a>
        </div>

        <Link href="/app" className={styles.launchButton}>
          Launch App <span>↗</span>
        </Link>
      </nav>

      <section className={styles.hero}>
        <div className={styles.heroCopy}>
          <p className={styles.eyebrow}>
            TOKENIZED EQUITY CREDIT INFRASTRUCTURE
          </p>

          <h1>
            Liquidity-aware credit for{" "}
            <em>tokenized equities.</em>
          </h1>

          <p className={styles.heroText}>
            Unlock stablecoin liquidity from tokenized equity positions with
            borrowing limits that adapt to executable market depth.
          </p>

          <div className={styles.heroActions}>
            <Link href="/app" className={styles.primary}>
              Launch Osprey <span>↗</span>
            </Link>

            <a href="#risk" className={styles.secondary}>
              Explore Risk Engine <span>→</span>
            </a>
          </div>

          <div className={styles.integrations}>
            <span>
              <i /> Solana
            </span>
            <span>Meteora DLMM</span>
            <span>Token-2022</span>
          </div>
        </div>

        <div className={styles.visual} data-hero-visual>
          <div className={styles.glow} />

          <div className={styles.riskCard}>
            <header>
              <span>LIQUIDITY-AWARE CREDIT</span>
              <b>● LIVE</b>
            </header>

            <div className={styles.asset}>
              <div className={styles.assetMark}>AI</div>

              <div>
                <strong>ANTH / USDC</strong>
                <span>Tokenized equity collateral</span>
              </div>
            </div>

            <div className={styles.riskMetric}>
              <span>COLLATERAL VALUE</span>
              <strong>$10,000</strong>
            </div>

            <div className={styles.track}>
              <div style={{ width: "100%" }} />
            </div>

            <div className={styles.riskMetric}>
              <span>EXECUTABLE RECOVERY</span>
              <strong>89.91%</strong>
            </div>

            <div className={styles.track}>
              <div style={{ width: "89.91%" }} />
            </div>

            <div className={`${styles.riskMetric} ${styles.finalMetric}`}>
              <span>EFFECTIVE LTV</span>
              <strong>58.44%</strong>
            </div>

            <div className={styles.track}>
              <div style={{ width: "58.44%" }} />
            </div>

            <footer>
              <span>Meteora liquidity</span>
              <span>+</span>
              <span>Issuer risk</span>
              <span>→</span>
              <strong>Osprey Risk Engine</strong>
            </footer>
          </div>
        </div>
      </section>

      <section id="risk" className={styles.section} data-reveal>
        <p className={styles.eyebrow}>THE CREDIT PROBLEM</p>

        <div className={styles.sectionIntro}>
          <h2>
            Tokenized equities need a{" "}
            <em>different credit model.</em>
          </h2>

          <p>
            Traditional lending assumes collateral can be liquidated near its
            quoted value. For emerging tokenized assets, quoted price and
            executable liquidation value can be very different.
          </p>
        </div>

        <div className={styles.model}>
          <article>
            <span>MARKET MAXIMUM</span>
            <strong>65.00%</strong>
            <small>Static collateral ceiling</small>
          </article>

          <b>→</b>

          <article>
            <span>EXECUTABLE RECOVERY</span>
            <strong>89.91%</strong>
            <small>Observed liquidation liquidity</small>
          </article>

          <b>→</b>

          <article className={styles.highlight}>
            <span>LIQUIDITY-ADJUSTED LTV</span>
            <strong>58.44%</strong>
            <small>Borrowing power enforced onchain</small>
          </article>
        </div>
      </section>

      <section className={styles.section} data-reveal>
        <p className={styles.eyebrow}>
          BUILT FOR REAL LIQUIDATION CONDITIONS
        </p>

        <h2 className={styles.sectionTitle}>
          How Osprey protects credit.
        </h2>

        <div className={styles.features}>
          <article>
            <span>01</span>
            <h3>Executable liquidity</h3>
            <p>
              Borrowing capacity responds to executable reference liquidation
              output instead of spot price alone.
            </p>
          </article>

          <article>
            <span>02</span>
            <h3>Issuer risk</h3>
            <p>
              Asset-level ceilings constrain borrowing power when token
              administration introduces additional risk.
            </p>
          </article>

          <article>
            <span>03</span>
            <h3>Freshness protection</h3>
            <p>
              Risk-increasing actions require fresh liquidity data before
              additional credit can be created.
            </p>
          </article>

          <article>
            <span>04</span>
            <h3>Onchain enforcement</h3>
            <p>
              Effective LTV is enforced by the lending program during borrowing
              and collateral withdrawal.
            </p>
          </article>
        </div>
      </section>

      <section id="markets" className={styles.section} data-reveal>
        <div className={styles.marketHeading}>
          <div>
            <p className={styles.eyebrow}>MARKETS</p>
            <h2 className={styles.sectionTitle}>
              Tokenized equity credit markets.
            </h2>
          </div>

          <Link href="/app">View market →</Link>
        </div>

        <div className={styles.markets}>
          <Link href="/app" className={styles.marketCard}>
            <header>
              <div className={styles.asset}>
                <div className={styles.assetMark}>AI</div>

                <div>
                  <strong>Anthropic</strong>
                  <span>ANTH / USDC</span>
                </div>
              </div>

              <b className={styles.live}>● LIVE</b>
            </header>

            <div className={styles.marketMetrics}>
              <div>
                <span>Market maximum</span>
                <strong>65.00%</strong>
              </div>

              <div>
                <span>Issuer ceiling</span>
                <strong>60.00%</strong>
              </div>

              <div>
                <span>Effective LTV</span>
                <strong>58.44%</strong>
              </div>
            </div>

            <footer>
              <span>Meteora DLMM liquidity</span>
              <span>Open market →</span>
            </footer>
          </Link>

          <article className={`${styles.marketCard} ${styles.upcoming}`}>
            <header>
              <div className={styles.asset}>
                <div className={styles.openAi}>◎</div>

                <div>
                  <strong>OpenAI</strong>
                  <span>OPENAI / USDC</span>
                </div>
              </div>

              <b>COMING NEXT</b>
            </header>

            <p>
              Expansion market for additional tokenized equity collateral.
            </p>

            <footer>
              <span>Isolated credit market</span>
              <span>Planned</span>
            </footer>
          </article>
        </div>
      </section>

      <section id="infrastructure" className={styles.stack} data-reveal>
        <div>
          <p className={styles.eyebrow}>TOKENIZED CAPITAL STACK</p>
          <h2>From tokenized ownership to usable credit.</h2>
        </div>

        <div className={styles.stackFlow}>
          <article>
            <span>01</span>
            <strong>Tokenized Equities</strong>
            <small>Onchain financial assets</small>
          </article>

          <b>→</b>

          <article>
            <span>02</span>
            <strong>Liquidity Venues</strong>
            <small>Executable market depth</small>
          </article>

          <b>→</b>

          <article className={styles.stackOsprey}>
            <span>03</span>
            <strong>Osprey</strong>
            <small>Liquidity-aware credit</small>
          </article>

          <b>→</b>

          <article>
            <span>04</span>
            <strong>DeFi Capital</strong>
            <small>Stablecoin liquidity</small>
          </article>
        </div>
      </section>

      <section className={styles.cta} data-reveal>
        <OspreyLogo size={50} showWordmark={false} />

        <h2>
          Turn tokenized equity
          <br />
          into productive collateral.
        </h2>

        <p>
          Osprey connects asset liquidity, issuer risk and onchain credit
          enforcement into a single lending layer.
        </p>

        <Link href="/app" className={styles.primary}>
          Launch Osprey <span>↗</span>
        </Link>
      </section>

      <footer className={styles.siteFooter}>
        <OspreyLogo size={30} />

        <p>
          Liquidity-aware credit infrastructure for tokenized equities on
          Solana.
        </p>

        <Link href="/app">Launch App →</Link>
      </footer>
    </main>
  );
}
