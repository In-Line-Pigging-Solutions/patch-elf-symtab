#include <stdio.h>

volatile char greeting[16] = "hello, world!";

int
main(void)
{

	printf("%s", (const char *)greeting);
	return 0;
}
