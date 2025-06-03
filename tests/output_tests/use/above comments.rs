// test-kind: before-after

use b;
use a;

// hi

struct X;

use b;
use a;
// below

struct Y;

// :after:

use a;
use b;

// hi

struct X;

use a;
use b;
// below

struct Y;
