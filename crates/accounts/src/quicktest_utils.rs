use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct BigSmall<T>
where
    T: Clone + std::cmp::Ord + Arbitrary,
{
    pub big: T,
    pub small: T,
}

impl<T> Arbitrary for BigSmall<T>
where
    T: Clone + std::cmp::Ord + Arbitrary,
{
    fn arbitrary(g: &mut Gen) -> Self {
        // Altought very unlikely, it is possible
        // to never be able to generate these values
        for _ in 0..u16::MAX {
            let v1 = T::arbitrary(g);
            let v2 = T::arbitrary(g);
            let big = v1.clone().max(v2.clone());
            let small = v1.min(v2);
            if big > small {
                return Self { big, small };
            }
        }
        panic!(
            "Could not generate big/small for {}",
            std::any::type_name::<T>()
        );
    }
}
