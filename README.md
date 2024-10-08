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
- Added ``Silence`` sample that has no effect when mixing just for testing
- Will mix all the sounds and normalize the amplitude of the combined sample
- ``RunArgs::generate_arrays`` is still supported so you can view the mixed samples using ``tools/plot.py``
### RunArgs::generate_arrays extension new functionality
- Now also generates a ``samples.wav`` file in addition to the ``samples.txt`` so you can listen to the audio
### Configurable constants
Both of these optimizations are on by default just to make the code run fast. Seems to be fine but I haven't proven the correctness of them to myself so I made them toggleable.
- FAST_AMPLITUDE - Take a shortcut guess when calculating amplitude. Much faster and probably good enough
- CAP_ARRAY_GENERATION_SIZE - Another shortcut guess when calculating amplitude. Can stop giant sample expansions.