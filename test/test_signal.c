#include <signal.h>
#include <stdlib.h>

void fn(int sig) {
  (void)sig;
  exit(1);
}

int main() {
  signal(SIGINT, fn);
  signal(SIGTERM, SIG_IGN);
  while (1)
    ;
}
