import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';

const pageSchema = z.object({
  title: z.string().min(1),
  description: z.string().min(1),
  order: z.number().optional(),
  example: z.union([z.string(), z.literal(false)]).optional(),
  exampleKind: z.enum(['base', 'component']).optional(),
});

const docs = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './docs' }),
  schema: pageSchema,
});

const shell = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './shell' }),
  schema: pageSchema,
});

const base = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './base' }),
  schema: pageSchema,
});

const zhDocs = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './zh-CN/docs' }),
  schema: pageSchema,
});

const zhShell = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './zh-CN/shell' }),
  schema: pageSchema,
});

const zhBase = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './zh-CN/base' }),
  schema: pageSchema,
});

export const collections = {
  docs,
  shell,
  base,
  'zh-docs': zhDocs,
  'zh-shell': zhShell,
  'zh-base': zhBase,
};
