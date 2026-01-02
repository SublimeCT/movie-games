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
  mode: 'wizard';
  theme?: string;
  synopsis?: string; // Renamed from worldview
  genre?: string[]; // Added
  characters?: CharacterInput[];
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

interface GenerateResponseData {
  id: string;
  template: MovieTemplate;
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

  const data = await parseApiResponse<GenerateResponseData | MovieTemplate>(
    response,
  );

  if (data && typeof data === 'object') {
    if ('template' in data && 'id' in data) {
      const maybeId = (data as GenerateResponseData).id;
      const maybeTemplate = (data as GenerateResponseData).template;
      if (typeof maybeId === 'string' && maybeTemplate) {
        const template = maybeTemplate as MovieTemplate;
        template.requestId = maybeId;
        return template;
      }
    }

    if ('projectId' in data) {
      const template = data as MovieTemplate;
      console.warn('Backend returned raw template, requestId might be missing');
      return template;
    }
  }

  console.error('Invalid response data:', data);
  throw new Error('Invalid response format');
}

export interface ImportTemplateRequest {
  template: MovieTemplate;
  theme?: string;
  synopsis?: string;
  genre?: string[];
  characters?: CharacterInput[];
  language?: string;
}

export async function importGameTemplate(
  req: ImportTemplateRequest,
): Promise<MovieTemplate> {
  const response = await fetch(`${API_BASE}/import`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  });

  const data = await parseApiResponse<GenerateResponseData | MovieTemplate>(
    response,
  );

  if (data && typeof data === 'object') {
    if ('template' in data && 'id' in data) {
      const maybeId = (data as GenerateResponseData).id;
      const maybeTemplate = (data as GenerateResponseData).template;
      if (typeof maybeId === 'string' && maybeTemplate) {
        const t = maybeTemplate as MovieTemplate;
        t.requestId = maybeId;
        return t;
      }
    }

    if ('projectId' in data) {
      const t = data as MovieTemplate;
      return t;
    }
  }

  console.error('Invalid response data:', data);
  throw new Error('Invalid response format');
}

/**
 * 更新剧情的分享状态，并在分享成功时返回 shared_records 的 ID。
 */
export interface ShareGameResponse {
  sharedRecordId: string | null;
}

/**
 * 更新指定请求的分享状态。
 */
export async function shareGame(
  id: string,
  shared: boolean,
): Promise<ShareGameResponse> {
  const response = await fetch(`${API_BASE}/share`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ id, shared }),
  });
  return parseApiResponse<ShareGameResponse>(response);
}

/**
 * 更新指定 requestId 对应的剧情模板（写回数据库）。
 * @param id 生成记录 ID（requestId）
 * @param template 完整剧情模板
 */
export async function updateGameTemplate(
  id: string,
  template: MovieTemplate,
  source?: string,
): Promise<MovieTemplate> {
  const response = await fetch(`${API_BASE}/template/update`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ id, template, source }),
  });

  const data = await parseApiResponse<MovieTemplate>(response);
  if (data && !data.requestId) data.requestId = id;
  return data;
}

export interface DeleteTemplateResponse {
  deleted: boolean;
}

export async function deleteGameTemplate(
  id: string,
): Promise<DeleteTemplateResponse> {
  const response = await fetch(`${API_BASE}/template/delete`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ id }),
  });

  return parseApiResponse<DeleteTemplateResponse>(response);
}

export async function getSharedGame(id: string): Promise<MovieTemplate> {
  const response = await fetch(`${API_BASE}/play/${id}`);
  const data = await parseApiResponse<MovieTemplate>(response);
  // Ensure requestId is attached if not present in the stored json
  if (data && !data.requestId) {
    data.requestId = id;
  }
  return data;
}

/**
 * 历史记录列表项（仅包含列表展示所需的轻量字段）。
 */
export interface RecordsListItem {
  requestId: string;
  title: string;
  sharedAt: string;
  shared: boolean;
  synopsis: string;
  genre: string;
  language: string;
  playCount: number;
}

/**
 * 根据 shared_records 的 ID 列表获取历史记录列表。
 */
export async function listRecords(ids: string[]): Promise<RecordsListItem[]> {
  const response = await fetch(`${API_BASE}/records`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ ids }),
  });
  return parseApiResponse<RecordsListItem[]>(response);
}

/**
 * 获取某个 requestId 的分享元信息，用于判断是否为创建人及分享状态。
 */
export interface SharedRecordMeta {
  requestId: string;
  shared: boolean;
  sharedAt: string | null;
  isOwner: boolean;
}

/**
 * 查询指定 requestId 对应的分享元信息。
 */
export async function getSharedRecordMeta(
  id: string,
): Promise<SharedRecordMeta> {
  const response = await fetch(`${API_BASE}/records/meta/${id}`);
  return parseApiResponse<SharedRecordMeta>(response);
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
  genre?: string[],
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
      genre,
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
  genre?: string[],
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
      genre,
      language,
      apiKey,
      baseUrl,
      model,
    }),
  });

  return parseApiResponse<CharacterInput[]>(response);
}

export async function expandSynopsisPrompt(
  theme: string,
  currentSynopsis?: string,
  genre?: string[],
  language?: string,
): Promise<string> {
  const response = await fetch(`${API_BASE}/expand/worldview/prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      theme,
      synopsis: currentSynopsis,
      genre,
      language,
    }),
  });

  return parseApiResponse<string>(response);
}

export async function expandCharacterPrompt(
  theme: string,
  synopsis: string,
  existingCharacters: CharacterInput[],
  genre?: string[],
  language?: string,
): Promise<string> {
  const response = await fetch(`${API_BASE}/expand/character/prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      theme,
      worldview: synopsis,
      existingCharacters,
      genre,
      language,
    }),
  });

  return parseApiResponse<string>(response);
}
