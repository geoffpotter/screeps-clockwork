import { getRange } from "../../../src";


export function getPathCost(path: RoomPosition[]) {
    let cost = 0;
    for (let pos of path) {
        let terrain = Game.map.getRoomTerrain(pos.roomName);
        let terrain_type = terrain.get(pos.x, pos.y);
        if (terrain_type === TERRAIN_MASK_WALL) {
            return Infinity;
        }
        cost += terrain_type === TERRAIN_MASK_SWAMP ? 5 : 1;
    }
    return cost;
}

export function pathIsValid(path: RoomPosition[], origins: {pos: RoomPosition, range: number}[], goals: {pos: RoomPosition, range: number}[]) {
    let start_ok = origins.some(origin => getRange(path[0], origin.pos) <= origin.range);
    let end_ok = goals.some(goal => getRange(path[path.length - 1], goal.pos) <= goal.range);
    return start_ok && end_ok;
}
