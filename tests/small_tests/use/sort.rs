// test-kind: before-after

use a::b;
use b;
use a::a;
use a;
use crate::a;
use super::a;
use self::a;
use a::{aa, c, b};
use a::{b, a};
use ::a;
use a::*;

// :after:

use self::a;
use super::a;
use crate::a;
use ::a;
use a;
use a::a;
use a::b;
use a::*;
use a::{a, b};
use a::{aa, b, c};
use b;
