import Link from '@docusaurus/Link';

const REPO = 'https://github.com/havaianasdestruido/steamlogger';

/**
 * Badge linking a documentation page back to the file it documents.
 *
 * Usage in MDX (globally available, no import needed):
 *   <SourceLink path="src/config.rs" lines={263} />
 */
export default function SourceLink({path, lines, branch = 'main'}) {
  return (
    <Link className="sl-source-badge" to={`${REPO}/blob/${branch}/${path}`}>
      <span aria-hidden="true">{'{ }'}</span>
      <span>{path}</span>
      {lines ? <span>· {lines} lines</span> : null}
    </Link>
  );
}
