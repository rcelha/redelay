use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use skiplist::OrderedSkipList;
use std::fmt;
use std::marker::PhantomData;

// TODO docstring
pub fn ser_skiplist<T, S>(skiplist: &OrderedSkipList<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    let mut state = serializer.serialize_seq(Some(skiplist.len()))?;
    for i in skiplist {
        state.serialize_element(i)?;
    }
    state.end()
}

// TODO docstring
pub fn de_skiplist<'de, T, D>(deserializer: D) -> Result<OrderedSkipList<T>, D::Error>
where
    T: Deserialize<'de> + Ord,
    D: Deserializer<'de>,
{
    struct OrderedSkipListVisitor<T>(PhantomData<fn() -> OrderedSkipList<T>>);

    impl<'de, T> Visitor<'de> for OrderedSkipListVisitor<T>
    where
        T: Deserialize<'de> + Ord,
    {
        type Value = OrderedSkipList<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a nonempty sequence of tuples")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut ret = if let Some(size_hint) = seq.size_hint() {
                OrderedSkipList::with_capacity(size_hint)
            } else {
                OrderedSkipList::new()
            };
            while let Some(t) = seq.next_element()? {
                ret.insert(t);
            }
            Ok(ret)
        }
    }
    let visitor = OrderedSkipListVisitor(PhantomData);
    deserializer.deserialize_seq(visitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        #[derive(Serialize)]
        struct OrderedSkipListWrapper(
            #[serde(serialize_with = "ser_skiplist")] OrderedSkipList<(u8, u16, u32)>,
        );

        let mut list = OrderedSkipList::new();
        list.insert((10, 10, 10));
        list.insert((5, 5, 5));
        list.insert((1, 1, 1));

        let wrapped_list = OrderedSkipListWrapper(list);

        let ser_list = serde_json::to_string(&wrapped_list).unwrap();
        assert_eq!(ser_list, "[[1,1,1],[5,5,5],[10,10,10]]");
    }

    #[test]
    fn deserialize() {
        #[derive(Deserialize)]
        struct OrderedSkipListWrapper(
            #[serde(deserialize_with = "de_skiplist")] OrderedSkipList<(u8, u16, u32)>,
        );

        let mut wrapped_list: OrderedSkipListWrapper =
            serde_json::from_str("[[1,1,1],[5,5,5],[10,10,10]]").unwrap();
        assert_eq!(wrapped_list.0.pop_front(), Some((1, 1, 1)));
        assert_eq!(wrapped_list.0.pop_front(), Some((5, 5, 5)));
        assert_eq!(wrapped_list.0.pop_front(), Some((10, 10, 10)));
        assert_eq!(wrapped_list.0.pop_front(), None);
    }
}
