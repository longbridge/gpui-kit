/** Shared by the hydrated UI and tests; never changes editorial featured order. */
export function selectShowcases(apps, { query = '', category = 'all', sort = 'newest' } = {}) {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const matches = apps.filter(app => {
    const text = [app.name, app.author ?? "", app.id, app.description, ...app.platforms, app.source ?? ''].join(' ').toLocaleLowerCase();
    return (category === 'all' || app.category === category) && terms.every(term => text.includes(term));
  });
  const featured = matches.filter(app => app.featured);
  const community = matches.filter(app => !app.featured);
  const compare = (a, b) => {
    if (sort === 'stars' && a.stars !== b.stars) return (b.stars ?? -1) - (a.stars ?? -1);
    return (Date.parse(b.publishedAt) || 0) - (Date.parse(a.publishedAt) || 0) || a.name.localeCompare(b.name);
  };
  community.sort(compare);
  return { featured, community, all: [...matches].sort(compare) };
}

/** Six editorial selections per page; browsing never changes catalog order. */
export function paginateFeatured<T extends { featured: boolean }>(apps: T[], requestedPage = 1) {
  return paginate(apps.filter(app => app.featured), requestedPage, 6);
}

/** Nine filtered and sorted catalog entries per page. */
export function paginateAll<T>(apps: T[], requestedPage = 1) {
  return paginate(apps, requestedPage, 9);
}

function paginate<T>(apps: T[], requestedPage: number, pageSize: number) {
  const totalPages = Math.ceil(apps.length / pageSize);
  const page = Math.max(1, Math.min(requestedPage, totalPages));
  return { apps: apps.slice((page - 1) * pageSize, page * pageSize), page, totalPages, total: apps.length };
}
