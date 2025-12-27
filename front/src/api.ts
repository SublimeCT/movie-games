import type { MovieTemplate } from './types/movie';

// 生产环境使用 /api 前缀 (由 Nginx 转发)
// 开发环境通过 Vite 代理转发 /api -> 后端端口
const API_BASE = '/api';

export class ApiError extends Error {
  status: number;
  code: string;
  payload?: unknown;

  constructor(
    status: number,
    message: string,
    code: string,
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

// 统一 API 响应格式
type ApiResponse<T> = {
  code: string;
  msg: string;
  data?: T;
};

type ErrorResponse = {
  code: string;
  msg: string;
};

async function parseApiError(response: Response): Promise<ApiError> {
  const text = await response.text();
  try {
    const json = JSON.parse(text) as ErrorResponse;
    const message = json.msg || text || `API Error: ${response.status}`;
    const code = json.code || 'UNKNOWN_ERROR';
    return new ApiError(response.status, message, code, json);
  } catch {
    return new ApiError(
      response.status,
      text || `API Error: ${response.status}`,
      'PARSE_ERROR',
    );
  }
}

async function parseApiResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw await parseApiError(response);
  }
  const json = (await response.json()) as ApiResponse<T>;
  // code = "0" 表示成功
  if (json.code === '0' && json.data !== undefined) {
    return json.data;
  }
  // 如果 code 不是 "0"，视为错误
  throw new ApiError(response.status, json.msg || 'Unknown error', json.code);
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

  return parseApiResponse<MovieTemplate>(response);
}

export async function generatePrompt(req: GenerateRequest): Promise<string> {
  const response = await fetch(`${API_BASE}/generate/prompt`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  });

  return parseApiResponse<string>(response);
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

  return parseApiResponse<string>(response);
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

  return parseApiResponse<CharacterInput[]>(response);
}
