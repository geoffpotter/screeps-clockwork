import { fromPacked } from '../../../src/utils/fromPacked';



interface PositionsForRoom {
  walkable: RoomPosition[];
  blocked: RoomPosition[];
  timestamp: number;
}

interface RoomPositionsCache {
  [roomName: string]: {
    walkable: number[];
    blocked: number[];
  };
}

export interface RoomPositions {
  walkable: RoomPosition[];
  blocked: RoomPosition[];
  timestamp: number;
  config: {
    topLeftRoom: string;
    bottomRightRoom: string;
    positionsPerRoom: number;
  }
}

declare global {
  interface Memory {
    RoomPositionsCache?: RoomPositionsCache;
  }
}

function parseRoomName(roomName: string): { x: number, y: number } {
  const match = roomName.match(/^([WE])(\d+)([NS])(\d+)$/);
  if (!match) {
    throw new Error(`Invalid room name: ${roomName}`);
  }
  
  const [, xDir, xVal, yDir, yVal] = match;
  const xx = parseInt(xVal);
  const yy = parseInt(yVal);

  // From room_index.rs:
  // For `Wxx` rooms, `room_x = -xx - 1`. For `Exx` rooms, `room_x = xx`.
  // For `Nyy` rooms, `room_y = -yy - 1`. For `Syy` rooms, `room_y = yy`.
  const x = xDir === 'W' ? -xx - 1 : xx;
  const y = yDir === 'N' ? -yy - 1 : yy;
  
  console.log(`Parsed ${roomName}: dir=${xDir}${yDir}, val=${xVal}${yVal} -> coords=(${x},${y})`);
  return { x, y };
}

function test(roomName: string) {
  const { x, y } = parseRoomName(roomName);
  return `${x},${y}`;
}

function getRoomNamesBetween(topLeftRoom: string, bottomRightRoom: string): string[] {
  const topLeft = parseRoomName(topLeftRoom);
  const bottomRight = parseRoomName(bottomRightRoom);
  
  const minX = Math.min(topLeft.x, bottomRight.x);
  const maxX = Math.max(topLeft.x, bottomRight.x);
  const minY = Math.min(topLeft.y, bottomRight.y);
  const maxY = Math.max(topLeft.y, bottomRight.y);
  // console.log(`Generating positions for rooms between ${topLeftRoom} and ${bottomRightRoom}`);
  // console.log(`MinX: ${minX}, MaxX: ${maxX}, MinY: ${minY}, MaxY: ${maxY}`);

  // console.log("you're dumb", 
  //   "W10N10", test("W10N10"), 
  //   "E10S10", test("E10S10"), 
  //   "W10S10", test("W10S10"), 
  //   "E10N10", test("E10N10"),
  //   "W0N0", test("W0N0"),
  //   "E0S0", test("E0S0"),
  //   "W0S0", test("W0S0"),
  //   "E0N0", test("E0N0")
  // )

  const rooms: string[] = [];
  
  for (let y = minY; y <= maxY; y++) {
    for (let x = minX; x <= maxX; x++) {
      // From room_index.rs:
      // For `Wxx` rooms, `room_x = -xx - 1`. For `Exx` rooms, `room_x = xx`.
      // For `Nyy` rooms, `room_y = -yy - 1`. For `Syy` rooms, `room_y = yy`.
      
      // For W rooms: -xx - 1 = x, so xx = -(x + 1)
      // For E rooms: xx = x
      const xPrefix = x < 0 ? 'W' : 'E';
      const xNum = x < 0 ? -(x + 1) : x;
      
      // For N rooms: -yy - 1 = y, so yy = -(y + 1)
      // For S rooms: yy = y
      const yPrefix = y < 0 ? 'N' : 'S';
      const yNum = y < 0 ? -(y + 1) : y;
      
      const roomName = `${xPrefix}${xNum}${yPrefix}${yNum}`;
      // console.log(`Coords (${x},${y}) -> Room ${roomName} [${xPrefix}${xNum}${yPrefix}${yNum}]`);
      rooms.push(roomName);
    }
  }

  // console.log(`Generated ${rooms.length} rooms: ${rooms.join(', ')}`);
  return rooms;
}

function generateRoomPositions(roomName: string): PositionsForRoom {
  const terrain = new Room.Terrain(roomName);
  const walkablePositions = new Set<number>();
  const blockedPositions = new Set<number>();

  // First pass: find walkable positions
  for (let x = 1; x < 49; x++) {
    for (let y = 1; y < 49; y++) {
      const pos = new RoomPosition(x, y, roomName) as any;
      if (terrain.get(x, y) !== TERRAIN_MASK_WALL) {
        walkablePositions.add(pos.__packedPos);
      }
    }
  }

  // Second pass: find blocked positions adjacent to walkable ones
  for (let x = 1; x < 49; x++) {
    for (let y = 1; y < 49; y++) {
      if (terrain.get(x, y) === TERRAIN_MASK_WALL) {
        const pos = new RoomPosition(x, y, roomName) as any;
        // Check if adjacent to walkable
        for (let dx = -1; dx <= 1; dx++) {
          for (let dy = -1; dy <= 1; dy++) {
            const nx = x + dx;
            const ny = y + dy;
            if (nx >= 1 && nx < 49 && ny >= 1 && ny < 49) {
              const nPos = new RoomPosition(nx, ny, roomName) as any;
              if (walkablePositions.has(nPos.__packedPos)) {
                blockedPositions.add(pos.__packedPos);
                break;
              }
            }
          }
        }
      }
    }
  }

  // Convert to arrays and shuffle immediately
  const walkable = shufflePositions(Array.from(walkablePositions));
  const blocked = shufflePositions(Array.from(blockedPositions));

  return {
    walkable: walkable.map(packedPos => (fromPacked(packedPos))),
    blocked: blocked.map(packedPos => (fromPacked(packedPos))),
    timestamp: Game.time
  };
}


function fillCache(rooms: string[]) {
  // Initialize cache if it doesn't exist
  if (!Memory.RoomPositionsCache) {
    Memory.RoomPositionsCache = {};
  }
  // Generate or fetch positions for each room
  for (const roomName of rooms) {
    if (!Memory.RoomPositionsCache[roomName]) {
      const positions = generateRoomPositions(roomName);
      Memory.RoomPositionsCache[roomName] = {
        // @ts-ignore
        walkable: positions.walkable.map(p => p.__packedPos),
        // @ts-ignore
        blocked: positions.blocked.map(p => p.__packedPos)
      };
    }
  }
}


export function getPositionsInRoom(roomName: string, count: number, type: 'walkable' | 'blocked', forceRegenerate?: boolean) {
  if (!Memory.RoomPositionsCache || forceRegenerate) {
    Memory.RoomPositionsCache = {};
  }
  fillCache([roomName]);
  let cachedPositions = Memory.RoomPositionsCache[roomName];
  if (type === 'walkable') {
    return cachedPositions.walkable.slice(0, count).map(packedPos => (fromPacked(packedPos)));
  } else {
    return cachedPositions.blocked.slice(0, count).map(packedPos => (fromPacked(packedPos)));
  }
}

export function getPositionsInArea(options: {
  topLeftRoom: string;
  bottomRightRoom: string;
  positionsPerRoom: number;
  forceRegenerate?: boolean;
}): RoomPositions {
  // clear 
  if (!Memory.RoomPositionsCache || options.forceRegenerate) {
    Memory.RoomPositionsCache = {};
  }
  // Get all rooms in the requested area
  const rooms = getRoomNamesBetween(options.topLeftRoom, options.bottomRightRoom);
  fillCache(rooms);

  // Compose the final result from the cache
  const result: RoomPositions = {
    walkable: [],
    blocked: [],
    timestamp: Game.time,
    config: {
      topLeftRoom: options.topLeftRoom,
      bottomRightRoom: options.bottomRightRoom,
      positionsPerRoom: options.positionsPerRoom
    }
  };

  // Take positions from each room, ensuring we get a mix
  const positionsPerRoomActual = Math.max(1, Math.floor(options.positionsPerRoom / rooms.length));
  for (const roomName of rooms) {
    const roomPositions = Memory.RoomPositionsCache[roomName];
    if (roomPositions) {
      if (roomPositions.walkable.length > 0) {
        result.walkable.push(...roomPositions.walkable
          .slice(0, positionsPerRoomActual)
          .map(packedPos => (fromPacked(packedPos))));
      }
      if (roomPositions.blocked.length > 0) {
        result.blocked.push(...roomPositions.blocked
          .slice(0, positionsPerRoomActual)
          .map(packedPos => (fromPacked(packedPos))));
      }
    }
  }

  // Shuffle the final arrays to ensure random distribution across rooms
  result.walkable = shufflePositions(result.walkable);
  result.blocked = shufflePositions(result.blocked);

  return result;
}

// Deterministic shuffle using a simple seeded random number generator
function shufflePositions<T extends RoomPosition | number>(positions: T[]): T[] {
  // Simple multiplicative hash for seeding
  function hash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash) + str.charCodeAt(i);
      hash = hash & hash; // Convert to 32-bit integer
    }
    return hash;
  }

  // Simple seeded random number generator
  function seededRandom(seed: number, index: number): number {
    const x = Math.sin(seed + index) * 10000;
    return x - Math.floor(x);
  }

  // Create a seed from the positions to ensure consistent shuffling
  const seed = positions.reduce((acc, pos) => {
    // @ts-ignore
    const value = typeof pos === 'number' ? pos : pos.__packedPos;
    return acc + hash(`${value}`);
  }, 0);

  // Fisher-Yates shuffle with seeded random
  const shuffled = [...positions];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(seededRandom(seed, i) * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled;
}

// Helper to convert to RoomPosition
export function toRoomPosition(pos: RoomPosition): RoomPosition {
  // @ts-ignore
  return fromPacked(pos.__packedPos);
} 
