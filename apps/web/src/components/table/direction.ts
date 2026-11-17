import type { PublicPlayer } from '@/generated/PublicPlayer'

/** Find the next occupied seat in the actual playing order, skipping eliminations. */
export function getNextActiveSeat(
  players: PublicPlayer[],
  currentSeat: number,
  direction: number,
): number | null {
  const seats = players
    .filter((player) => !player.eliminated)
    .map((player) => player.seat)
    .sort((a, b) => a - b)

  if (seats.length <= 1) return null
  const index = seats.indexOf(currentSeat)
  if (index >= 0) {
    return seats[(index + (direction >= 0 ? 1 : -1) + seats.length) % seats.length] ?? null
  }

  // The current player can be eliminated as a round ends. Choose the next
  // surviving seat in the same direction rather than displaying a dead seat.
  if (direction >= 0) return seats.find((seat) => seat > currentSeat) ?? seats[0] ?? null
  return [...seats].reverse().find((seat) => seat < currentSeat) ?? seats[seats.length - 1] ?? null
}
