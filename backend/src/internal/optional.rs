pub trait Optional<T> = Into<Option<T>> + std::marker::Send;
