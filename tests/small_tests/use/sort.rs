// test-kind: before-after

use a::b;
use b;
use a::a;
use a;
use a::{aa, c, b};
use a::{b, a};
use a::*;

// :after:

use a;
use a::a;
use a::b;
use a::*;
use a::{a, b};
use a::{aa, b, c};
use b;
