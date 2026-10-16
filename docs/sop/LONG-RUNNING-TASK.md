# Long-Running Task

Use for work spanning three or more ordered phases, multiple providers/systems, long unattended jobs, or durable checkpoints.

Before execution, inventory all known phases, dependencies, permissions and decision points. Seek one consolidated confirmation for the bounded plan instead of interrupting after every routine step. Preserve a checkpoint after each completed phase before starting the next. Record background job IDs/state and never busy-poll when event/long-poll completion is available. On interruption, leave a resume note in the active iteration/Story with completed evidence, current state and exact next step.
