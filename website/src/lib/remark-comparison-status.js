import { visit } from 'unist-util-visit';

const statuses = {
  yes: { label: 'Supported', className: 'comparison-yes' },
  partial: { label: 'Partially supported', className: 'comparison-partial' },
  no: { label: 'Not supported', className: 'comparison-no' },
};

// Keep the comparison source readable while emitting an accessible status icon.
export function remarkComparisonStatus() {
  return (tree) => {
    visit(tree, 'html', (node) => {
      const match = /^<comparison-status value="([a-z]+)"( decorative)?><\/comparison-status>$/.exec(node.value.trim());
      if (!match) return;

      const status = statuses[match[1]];
      if (!status) throw new Error(`Unknown comparison status: ${match[1]}`);

      node.value = match[2]
        ? `<span class="${status.className}" aria-hidden="true"></span>`
        : `<span class="${status.className}" role="img" aria-label="${status.label}" data-status="${match[1]}"></span>`;
    });
  };
}
