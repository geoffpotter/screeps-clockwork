


export function drawPath(path: RoomPosition[], color: string = "#000000") {
    // break path into segments per room, display each segment as a poly
    Game.map.visual.poly(path, { stroke: color });
    let segments: RoomPosition[][] = [];
    let currentRoom = path[0].roomName;
    let currentSegment: RoomPosition[] = [];
    for (let i = 0; i < path.length; i++) {
        if (path[i].roomName !== currentRoom) {
            segments.push(currentSegment);
            currentSegment = [];
            currentRoom = path[i].roomName;
        }
        currentSegment.push(path[i]);
    }
    segments.push(currentSegment);
    for (let segment of segments) {
        new RoomVisual(segment[0].roomName).poly(segment, { stroke: color });

    }
}


export interface PathfindingResult {
    path: RoomPosition[];
    cost: number;
    ops: number;
    incomplete: boolean;
}

export interface PathfindingBenchmarkArgs {
    origins: RoomPosition[];
    goals: RoomPosition[];
}






