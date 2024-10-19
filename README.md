# Sound Tests
Various small test programs that I am creating to learn more about audio processing.
# Audio synthesis tests:
## 1: Hello Sine - Play a sine wave
- Creates a sine wave for one second on the default audio device
- Can change the frequency
- Very similar to the cpal example code
## 2: Samples - Play various samples
- Adds new functionality on to '1: Hello Sine'
- Can now create a Sine, Square, Sawtooth or Triangle sound
- RunArgs struct -- change the sample in the main fn, change the frequency
### RunArgs::generate_arrays
- If this boolean set true no sound will play
- Instead a text file ``samples.txt`` will be generated in the crate root
- The python script at ``tools/plot.py`` will open this data when run and plot the sample
- This lets you inspect and verify if the samples are correct
## 3: Mix - Combine the various samples
- Adds new functionality on to '2: Samples'
- Can now mix and play many samples at once
- RunArgs struct now takes a Vec of (Sample, Freq) pairs to specify the sounds you want to mix
- Added ``Silence`` waveform that has no effect when mixing just for testing
- Will mix all the sounds and normalize the amplitude of the combined sample
- ``RunArgs::generate_arrays`` is still supported so you can view the mixed samples using ``tools/plot.py``
### RunArgs::generate_arrays extension new functionality
- Now also generates a ``samples.wav`` file in addition to the ``samples.txt`` so you can listen to the audio
### Configurable constants
Both of these optimizations are on by default just to make the code run fast. Seems to be fine but I haven't proven the correctness of them to myself so I made them toggleable.
- FAST_AMPLITUDE - Take a shortcut guess when calculating amplitude. Much faster and probably good enough
- CAP_ARRAY_GENERATION_SIZE - Another shortcut guess when calculating amplitude. Can stop giant sample expansions.
## 4: FM Synth - Simple FM Synthesis
- Modifies '3: Mix' but is not additive, removes the ability to mix
- RunArgs changed. Takes a 'carrier' (Wave, Freq) for the base waveform and a list of modulators
- Modulators consist of (Wave, Freq, Depth)
- The final waveform is frequency modified (by the list of modulators, first to last)
- Modulators run in a linear chain progressively modifying each others output
- More complex FM synthesis chains/trees are possible but I've just done the simplest thing here
- Added a new waveform ``OnOff`` that is 1 for half its period than 0 for the rest (useful for modulators?)
- RunArgs ``hide_device_out`` will disable audio device information prints but not other information.. is overriden by ``quiet`` flag
- Also normalized the sine wave (can correctly generate sample values after 1sec of playback)
## FM Synth example
The following setup sounds like a harsher telephone ringing sound:
```rust
carrier: (WaveformKind::Sine, 440.0),
modulators: vec![
    (WaveformKind::Square, 1760.0, 22.0),
],
```
# Tools (python scripts)
- ``plot.py`` and ``plot2.py`` are interchangeable
- ``plot.py`` uses native desktop rendering
- ``plot2.py`` uses browser rendering
# Tests (as in not sure if they're good)
## 1: Lowpass
- Modify '3: Mix' and add a naive first order low pass filter
- Only parameter is a cutoff frequency
- Seems to impart noise and phase shift on the output
- Needs more research...
## 2: Play midi input
- Modify '3: Mix' adding midi reading
- Keep track of currently activated notes and their velocities, mixes them
- Scales the linear midi velocities into an exponential to guess/match perceived loudness
- Lets you pick your midi device and wave choice
- 'Unlimited' polyphony
- BASIC MIDI SUPPORT INCLUDES: channel 0, Commands: Note on, Note off
- I think it has a decent amount of delay from keypress -> note heard.. not sure
# Outdated documentation
## OLD FM synthesis examples
**WARNING:** These only apply to an older version (commit hash ``c4b68dcd108e497fe95b117fec56942d9af448b1``) and ``p4fmsynth`` was changed after.
Be sure to use the plotting tool to make sense of these, FM synthesis can be really hard to predict.
### 1 - Square into Sine
```rust
(WaveformKind::Square, 440.0),
(WaveformKind::Sine, 440.0),
```
Explained: When the square is 1 (first 1/2 of the cycle) it adds 440Hz to the sine making it 880Hz. But when it is 0 (second 1/2 of the cycle) it subtracts 440Hz making it 0Hz
### 2 - Square into Sine with wobble
```rust
(WaveformKind::Sine, 4.4),
(WaveformKind::Square, 440.0),
(WaveformKind::Sine, 440.0),
```
- Same as previous example *except* the 4.4Hz sine controlling the square makes the waveform 'wobble' 