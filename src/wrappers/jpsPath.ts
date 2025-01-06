import { ephemeral } from '../utils/cleanup';
import { MAX_USIZE } from '../utils/constants';
import { fromPackedRoomName } from '../utils/fromPacked';
import {
  ClockworkCostMatrix,
  js_jps_multiroom_distance_map,
  js_path_to_multiroom_distance_map_origin
} from '../wasm/screeps_clockwork';
import { ClockworkPath } from './path';
import { visualizeDistanceMap } from '../../test/visualizations/helpers/visualizeDistanceMap';
import { ClockworkMultiroomDistanceMap } from './multiroomDistanceMap';

/**
 * Create a distance map for the given start positions, using JPS.
 *
 * This calculates a distance map across multiple rooms, with a few configurable limits:
 * - `maxTiles`: The maximum number of tiles to explore.
 * - `maxTileDistance`: Don't explore tiles further (in Chebyshev distance) than this.
 *
 * At least one of these limits must be set.
 *
 * @param start - The starting positions.
 * @param options - The options for the distance map.
 * @returns A multi-room distance map.
 */
export function jpsPath(
  start: RoomPosition[],
  destinations: RoomPosition[],
  {
    costMatrixCallback,
    maxOps = MAX_USIZE
  }: {
    costMatrixCallback: (room: string) => ClockworkCostMatrix | undefined;
    maxOps?: number;
  }
) {
  if (!destinations?.length) {
    throw new Error('At least one destination must be set');
  }

  const startPacked = new Uint32Array(start.map(pos => pos.__packedPos));
  const destinationsPacked = new Uint32Array(destinations.map(pos => pos.__packedPos));
  const distanceMap = ephemeral(
    js_jps_multiroom_distance_map(
      startPacked,
      (room: number) => costMatrixCallback(fromPackedRoomName(room)),
      maxOps,
      destinationsPacked
    )
  );

  // const distanceMap_test = new ClockworkMultiroomDistanceMap(distanceMap);
  // for (const room of distanceMap_test.getRooms()) {
  //   visualizeDistanceMap(room, distanceMap_test.getRoom(room)!);
  // }
  return new ClockworkPath(js_path_to_multiroom_distance_map_origin(destinationsPacked[0], distanceMap));
}
