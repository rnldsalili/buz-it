export type ClientRole = "player" | "board";

export type ClientMessage =
  | {
      type: "hello";
      role: ClientRole;
      name?: string;
      playerId?: string | null;
      hostKey?: string;
    }
  | { type: "buzz" }
  | { type: "arm" }
  | { type: "reset" };

export type SnapshotPlace = {
  playerId: string;
  name: string;
  place: number;
};

export type SnapshotPlayer = {
  id: string;
  name: string;
  connected: boolean;
};

export type ServerMessage =
  | { type: "helloOk"; playerId?: string | null; role: ClientRole }
  | { type: "error"; code: string; message: string }
  | {
      type: "snapshot";
      accepting: boolean;
      roundId: number;
      players: SnapshotPlayer[];
      sequence: SnapshotPlace[];
      lanUrls: string[];
      you: { id?: string | null; role: ClientRole; place?: number | null };
    };
