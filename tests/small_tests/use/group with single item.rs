// test-kind: before-after

use foo::{ bar };
use foo::{ bar, };

// :after:

use foo::bar;
use foo::bar;
