import catalog from "../../data/standards/catalog.json";

export interface Standard {
  code: string;
  name: string;
  category: string;
  status: string;
  published: string;
  effective: string;
  url: string;
}

function normalize(text: string): string {
  return text.toLowerCase().replace(/\s/g, "").replace(/[－—–]/g, "-");
}

export const standards: Standard[] = catalog.standards;
const indexed = standards.map(standard => ({ standard, code: normalize(standard.code), name: normalize(standard.name) }));
export function searchStandards(query: string): Standard[] {
  const key = normalize(query);
  return key ? indexed.filter(item => item.code.includes(key) || item.name.includes(key)).map(item => item.standard) : [];
}
