#include <limits.h>
#include <stdio.h>

#define PERF_COUNT 1

#define SAMPLE_RATE 32768

// about 31 Hz (C1) (at 32k)
#define PHASE_MAX 1041
// about C2
#define PHASE_MIN 512

#define ERROR_T_MAX 4294967295

#define WINDOW (PHASE_MAX - PHASE_MIN)

typedef   signed char  sample_t;
typedef unsigned int   error_t;
typedef unsigned short phase_t;

#include "samples/bass.c"
#include "samples/voice.c"

#ifdef PERF_COUNT
int w_e_count = 0;
#endif

float freq(int phase) {
  return 1.0/(phase * (1.0 / SAMPLE_RATE));
}

float freq_f(float phase) {
  return 1.0/(phase / SAMPLE_RATE);
}

unsigned int sample_error(signed char a, signed char b) {
  //  return (unsigned int) ((signed int) a - (signed int) b) * ((signed int) a - (signed int) b);
  return (unsigned int) (a > b ? a - b : b - a);
}

error_t window_error(sample_t *data, phase_t offset, error_t limit) {
  error_t error = 0;
  for (int i = 0; i < WINDOW && error < limit; i++) {
    #ifdef PERF_COUNT
    w_e_count++;
    #endif
    error += sample_error(data[i], data[i+offset]);
  }
  return error;
}

phase_t phase(sample_t *data, int start, int end) {
  error_t min_error = window_error(data, start, ERROR_T_MAX);
  phase_t min_index = 0;
  for (int i = start+1; i < end; i++) {
    error_t current = window_error(data, i, min_error);
    if (current < min_error) {
      min_error = current;
      min_index = i;
    }
  }
  printf("min_error: %i\n", min_error);
  return min_index;
}

void average_phase(sample_t *data, int *offsets, int offsets_c) {
  phase_t sum = 0;
  for (int i = 0; i < offsets_c; i++) {
    phase_t phase_i = phase(data+offsets[i], PHASE_MIN, PHASE_MAX);
    sum += phase_i;
    printf("offset: %i, phase: %i, sum: %i\n", offsets[i], phase_i, sum);
  }
  printf("freq: %f\n", freq_f(sum/offsets_c));
}

int main() {
  /*
  int phase_sum = 0;
  for (int i = 0; i < 1000; i++) {
    phase_sum += phase(bass);
  }
  printf("%i", phase_sum);
  */
  //  printf("\n");
  //  printf("Bass phase: %i freq: %f\n", average_phase(bass), freq(phase(bass)));
  //  printf("Voice phase: %i freq: %f\n", phase(voice), freq(phase(voice)));
  //phase(bass);
  int offsets[4] = {0, 1015, 2320, 7060};
  average_phase(bass, offsets, 4);
  #ifdef PERF_COUNT
  printf("Window error loops: %i\n", w_e_count);
  #endif
  signed char a = -100;
  signed char b = 100;
  printf("d: %i", a - b);
  //  printf("ad: %i", abs(a-b));
}
