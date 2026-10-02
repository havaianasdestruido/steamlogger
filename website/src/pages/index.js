import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Heading from '@theme/Heading';
import HomepageFeatures from '@site/src/components/HomepageFeatures';

import styles from './index.module.css';

const sampleLog = `{
  "games": [
    {
      "name": "Team Fortress 2",
      "start": "2026-08-10T14:32:15-03:00",
      "end": "2026-08-10T16:47:03-03:00",
      "map": "cp_dustbowl",
      "server": "Team Fortress 2 #42",
      "friends_playing": ["medicmain99"],
      "friends_in_lobby": ["medicmain99"]
    }
  ]
}`;

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className={clsx('container', styles.heroInner)}>
        <div className={styles.heroText}>
          <Heading as="h1" className={styles.heroTitle}>
            {siteConfig.title}
          </Heading>
          <p className={styles.heroSubtitle}>{siteConfig.tagline}</p>
          <p className={styles.heroBlurb}>
            A single-binary Rust daemon that polls the Steam Web API, detects
            when a game starts and stops, and records each session — plus who
            you played with and where — into one atomically written JSON file.
          </p>
          <div className={styles.buttons}>
            <Link className="button button--primary button--lg" to="/docs/intro">
              Read the docs
            </Link>
            <Link
              className="button button--secondary button--lg"
              to="/docs/getting-started/quickstart">
              Quickstart
            </Link>
            <Link
              className="button button--secondary button--outline button--lg"
              to="/docs/reference">
              Code reference
            </Link>
          </div>
        </div>
        <div className={styles.heroCode}>
          <CodeBlock language="json" title="steamlog.json">
            {sampleLog}
          </CodeBlock>
        </div>
      </div>
    </header>
  );
}

function QuickStart() {
  return (
    <section className={styles.quickstart}>
      <div className="container">
        <div className="row">
          <div className="col col--4">
            <Heading as="h2">Up and running in three commands</Heading>
            <p>
              Copy the example config, drop in your{' '}
              <Link href="https://steamcommunity.com/dev/apikey">
                Steam Web API key
              </Link>{' '}
              and SteamID64, then let it poll. Everything else has a sensible
              default.
            </p>
            <Link
              className="button button--primary"
              to="/docs/getting-started/installation">
              Installation guide
            </Link>
          </div>
          <div className="col col--8">
            <CodeBlock language="bash">
              {`cp steamlogger.example.toml steamlogger.toml
$EDITOR steamlogger.toml      # api_key + steam_id
cargo run --release           # Ctrl+C to stop`}
            </CodeBlock>
          </div>
        </div>
      </div>
    </section>
  );
}

export default function Home() {
  return (
    <Layout
      title="Steam session logger"
      description="Documentation for SteamLogger: a Rust app that logs every Steam game session, with friends, lobby, server and map enrichment, to a JSON file.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
        <QuickStart />
      </main>
    </Layout>
  );
}
