/** Sequence gate for absolute control snapshots, which may safely supersede gaps. */
export class ControlSequenceGate {
  private applied = 0;

  reset(): void {
    this.applied = 0;
  }

  accept(sequence: number): { accepted: boolean; appliedSequence: number } {
    if (!Number.isSafeInteger(sequence) || sequence <= 0) {
      return { accepted: false, appliedSequence: this.applied };
    }
    if (sequence <= this.applied) return { accepted: false, appliedSequence: this.applied };
    this.applied = sequence;
    return { accepted: true, appliedSequence: this.applied };
  }

  get appliedSequence(): number {
    return this.applied;
  }
}
