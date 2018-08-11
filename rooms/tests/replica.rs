use rooms::prelude32::*;

#[test]
fn base() {
    let mut replica = Replica::new(View::Within(10.0));

    {
        replica.extend(vec![1, 2, 3]);
        assert_eq!(&replica.created(), &[1, 2, 3]);
        assert_eq!(&replica.removed(), &[]);
        assert_eq!(&replica.nchange(), &[]);
    }

    {
        replica.extend(vec![4, 2, 3, 4]);
        assert_eq!(&replica.created(), &[4]);
        assert_eq!(&replica.removed(), &[1]);
        assert_eq!(&replica.nchange(), &[2, 3]);
    }

    {
        replica.extend(vec![4, 2, 3, 4]);
        assert_eq!(&replica.created(), &[]);
        assert_eq!(&replica.removed(), &[]);
        assert_eq!(&replica.nchange(), &[2, 3, 4]);
    }

    {
        replica.extend(std::iter::empty());
        assert_eq!(&replica.created(), &[]);
        assert_eq!(&replica.removed(), &[2, 3, 4]);
        assert_eq!(&replica.nchange(), &[]);
    }
}
