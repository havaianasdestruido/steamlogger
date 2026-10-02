import clsx from 'clsx';
import Link from '@docusaurus/Link';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

const FeatureList = [
  {
    title: 'Session tracking that survives restarts',
    to: '/docs/architecture/session-lifecycle',
    description: (
      <>
        A small state machine turns a stream of polls into start/switch/end
        events. Restart mid-game and the open session (<code>end: null</code>)
        is picked up where it left off instead of being duplicated.
      </>
    ),
  },
  {
    title: 'Friends, lobby, server and map',
    to: '/docs/architecture/enrichment-pipeline',
    description: (
      <>
        Optional enrichment answers who else was online in the same game, who
        shared your exact lobby, and which server and map you were on — via the
        Steam Web API plus a direct A2S_INFO query.
      </>
    ),
  },
  {
    title: 'Degrades instead of failing',
    to: '/docs/architecture/resilience',
    description: (
      <>
        Every enrichment source is best effort. If Steam is down or the server
        never answers, the base <code>name</code>/<code>start</code>/
        <code>end</code> log is still written — atomically, on every poll.
      </>
    ),
  },
  {
    title: 'Documented down to the last public item',
    to: '/docs/reference',
    description: (
      <>
        The code reference covers all nine modules: every public struct, enum,
        and function with its signature, behaviour, errors, and the tests that
        pin it down.
      </>
    ),
  },
  {
    title: 'Plain JSON you already know how to query',
    to: '/docs/output-format',
    description: (
      <>
        One <code>steamlog.json</code> file, one documented schema, no database.
        Pipe it through <code>jq</code> for playtime totals or import it
        anywhere.
      </>
    ),
  },
  {
    title: 'Tested without touching the network',
    to: '/docs/contributing/testing',
    description: (
      <>
        Unit and integration tests cover config loading, API parsing, the A2S
        protocol (against a loopback UDP server), the state machine, and atomic
        persistence.
      </>
    ),
  },
];

function Feature({title, description, to}) {
  return (
    <div className={clsx('col col--4')}>
      <Link to={to} className={styles.card}>
        <Heading as="h3" className={styles.cardTitle}>
          {title}
        </Heading>
        <p className={styles.cardBody}>{description}</p>
      </Link>
    </div>
  );
}

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
