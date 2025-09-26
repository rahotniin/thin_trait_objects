use crate::prelude::*;

type Unit = ();

type Ref<'a, T> = &'a T;
type Mut<'a, T> = &'a mut T;

type Array<T, const N: usize> = [T; N];

type Slice<T> = [T];

type Tuple1<T1>                                             = (T1,);
type Tuple2<T1, T2>                                         = (T1, T2);
type Tuple3<T1, T2, T3>                                     = (T1, T2, T3);
type Tuple4<T1, T2, T3, T4>                                 = (T1, T2, T3, T4);
type Tuple5<T1, T2, T3, T4, T5>                             = (T1, T2, T3, T4, T5);
type Tuple6<T1, T2, T3, T4, T5, T6>                         = (T1, T2, T3, T4, T5, T6);
type Tuple7<T1, T2, T3, T4, T5, T6, T7>                     = (T1, T2, T3, T4, T5, T6, T7);
type Tuple8<T1, T2, T3, T4, T5, T6, T7, T8>                 = (T1, T2, T3, T4, T5, T6, T7, T8);
type Tuple9<T1, T2, T3, T4, T5, T6, T7, T8, T9>             = (T1, T2, T3, T4, T5, T6, T7, T8, T9);
type TupleA<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA>         = (T1, T2, T3, T4, T5, T6, T7, T8, T9, TA);
type TupleB<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB>     = (T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB);
type TupleC<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB, TC> = (T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB, TC);

impl_stable_any! {
    Unit;

    u8; u16; u32; u64; u128; usize;
    i8; i16; i32; i64; i128; isize;

    f32; f64;

    bool;

    char;

    Ref<'a, T>;
    Mut<'a, T>;

    Slice<T>;

    Array<T, const N: usize>;

    Tuple1<T1>;
    Tuple2<T1, T2>;
    Tuple3<T1, T2, T3>;
    Tuple4<T1, T2, T3, T4>;
    Tuple5<T1, T2, T3, T4, T5>;
    Tuple6<T1, T2, T3, T4, T5, T6>;
    Tuple7<T1, T2, T3, T4, T5, T6, T7>;
    Tuple8<T1, T2, T3, T4, T5, T6, T7, T8>;
    Tuple9<T1, T2, T3, T4, T5, T6, T7, T8, T9>;
    TupleA<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA>;
    TupleB<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB>;
    TupleC<T1, T2, T3, T4, T5, T6, T7, T8, T9, TA, TB, TC>;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn array() {
        assert_ne!(<[u8; 1]>::UUID, <[u8; 2]>::UUID);
    }
}