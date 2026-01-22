import { openDB, type DBSchema, type IDBPDatabase } from 'idb';
import type { MovieTemplate, Ending } from '../types/movie';
import type { Story } from '../types/Story';

export interface GameState {
  gameId: string;
  currentNodeId: string;
  playerState: { flags: Record<string, boolean>; variables: Record<string, any> };
  historyStack: { nodeId: string; state: any }[];
  ending?: Ending;
}

interface MovieGamesDB extends DBSchema {
  played_games: {
    key: string;
    value: (MovieTemplate | Story) & {
      _localId: string; // Internal ID for IDB
      lastPlayedAt: number;
      source: 'owner' | 'shared' | 'import';
    };
    indexes: { 'by-lastPlayedAt': number };
  };
  draft: {
    key: string;
    value: MovieTemplate;
  };
  active_session: {
    key: string; // 'current'
    value: GameState;
  };
}

const DB_NAME = 'movie_games_db';
const DB_VERSION = 1;

let dbPromise: Promise<IDBPDatabase<MovieGamesDB>>;

function getDB() {
  if (!dbPromise) {
    dbPromise = openDB<MovieGamesDB>(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains('played_games')) {
          const store = db.createObjectStore('played_games', { keyPath: '_localId' });
          store.createIndex('by-lastPlayedAt', 'lastPlayedAt');
        }
        if (!db.objectStoreNames.contains('draft')) {
          // No keyPath, so we can use a fixed key 'current'
          db.createObjectStore('draft');
        }
        if (!db.objectStoreNames.contains('active_session')) {
          db.createObjectStore('active_session');
        }
      },
    });
  }
  return dbPromise;
}

export const db = {
  // --- Draft ---
  async saveDraft(template: MovieTemplate) {
    const d = await getDB();
    await d.put('draft', template, 'current');
  },

  async getDraft(): Promise<MovieTemplate | undefined> {
    const d = await getDB();
    return d.get('draft', 'current');
  },

  async clearDraft() {
    const d = await getDB();
    await d.delete('draft', 'current');
  },

  // --- Played Games ---
  async savePlayedGame(template: MovieTemplate | Story, source: 'owner' | 'shared' | 'import') {
    const d = await getDB();
    const _localId = template.requestId || (template as MovieTemplate).projectId || (template as any).id || crypto.randomUUID();
    const record = {
      ...template,
      _localId,
      lastPlayedAt: Date.now(),
      source,
    };
    await d.put('played_games', record);
    return _localId;
  },

  async getPlayedGame(id: string) {
    const d = await getDB();
    return d.get('played_games', id);
  },

  async getAllPlayedGames() {
    const d = await getDB();
    return d.getAllFromIndex('played_games', 'by-lastPlayedAt');
  },

  async deletePlayedGame(id: string) {
    const d = await getDB();
    await d.delete('played_games', id);
  },

  // --- Active Session ---
  async setActiveSession(state: GameState) {
    const d = await getDB();
    await d.put('active_session', state, 'current');
  },

  async getActiveSession(): Promise<GameState | undefined> {
    const d = await getDB();
    return d.get('active_session', 'current');
  },

  async clearActiveSession() {
    const d = await getDB();
    await d.delete('active_session', 'current');
  },
};
