type Index = u64;
type Term  = u64;

pub trait Tracker {
    type Entity;

    fn get_last_log_index() -> Index;

    fn get_last_log_term() -> Term;

    fn get_last_commited_index() -> Index;

    fn get_log_entity(index: Index) -> Self::Entity;

    fn get_log_term(index: Index) -> Term;

    fn append_log(entity: Self::Entity, term: u64) -> crate::Result<Index>;

    fn delete_last_log() -> crate::Result<()>;

    fn commit() -> crate::Result<()>;
}
