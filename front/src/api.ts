import type { MovieTemplate } from './types/movie';

// 生产环境使用 /api 前缀 (由 Nginx 转发)
// 开发环境通过 Vite 代理转发 /api -> 后端端口
const API_BASE = '/api';

export class ApiError extends Error {
  status: number;
  code?: string;
  payload?: unknown;

  constructor(
    status: number,
    message: string,
    code?: string,
    payload?: unknown,
  ) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    this.payload = payload;
  }
}

export interface GenerateRequest {
  mode: 'wizard' | 'free';
  theme?: string;
  synopsis?: string; // Renamed from worldview
  genre?: string[]; // Added
  characters?: CharacterInput[];
  freeInput?: string;
  language?: string;
  size?: '1024x1024' | '864x1152' | '1152x864';
  apiKey?: string;
  baseUrl?: string;
  model?: string;
}

type ErrorPayload = {
  error?: string;
  code?: string;
  activeRequests?: number;
};

async function parseApiError(response: Response): Promise<ApiError> {
  const text = await response.text();
  try {
    const json = JSON.parse(text) as ErrorPayload;
    const message = json.error || text || `API Error: ${response.status}`;
    return new ApiError(response.status, message, json.code, json);
  } catch {
    return new ApiError(
      response.status,
      text || `API Error: ${response.status}`,
    );
  }
}

export interface CharacterInput {
  name: string;
  description: string;
  gender: string;
  isMain: boolean;
}

export async function generateGame(
  req: GenerateRequest,
): Promise<MovieTemplate> {
  const response = await fetch(`${API_BASE}/generate`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  });

  if (!response.ok) throw await parseApiError(response);

  return response.json();
}

export async function generatePrompt(req: GenerateRequest): Promise<string> {
  const response = await fetch(`${API_BASE}/generate/prompt`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  });

  if (!response.ok) throw await parseApiError(response);
  const data = (await response.json()) as { prompt?: string };
  return data.prompt || '';
}

export async function expandSynopsis(
  theme: string,
  currentSynopsis?: string,
  language?: string,
  apiKey?: string,
  baseUrl?: string,
  model?: string,
): Promise<string> {
  const response = await fetch(`${API_BASE}/expand/worldview`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      theme,
      synopsis: currentSynopsis,
      language,
      apiKey,
      baseUrl,
      model,
    }),
  });
  if (!response.ok) throw await parseApiError(response);
  const data = (await response.json()) as { worldview: string };
  return data.worldview;
}

export async function expandCharacter(
  theme: string,
  synopsis: string,
  existingCharacters: CharacterInput[],
  language?: string,
  apiKey?: string,
  baseUrl?: string,
  model?: string,
): Promise<CharacterInput[]> {
  const response = await fetch(`${API_BASE}/expand/character`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      theme,
      worldview: synopsis,
      existingCharacters,
      language,
      apiKey,
      baseUrl,
      model,
    }),
  });
  if (!response.ok) throw await parseApiError(response);
  const data = await response.json();
  return data.characters;
}
