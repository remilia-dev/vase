#define BLANK
#define X(Y) Y
#define BEGIN {
#define RET_X return X
#define END }

BLANK
int main() BEGIN
    RET_X(1);
END