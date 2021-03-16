# Lo(cal) Di(ctionary) L(ibrary)

`lodil` is a library that kind of reminds me of [Redis, the Re(mote) Di(ctionary) S(erver)](redis.io).
Except it's local. And it's a library. And it... was written as an exercise.

**Note**: _for a better experience reading this README, and the code-level
documentation, run `cargo doc --no-deps` to generate a rustdoc and then navigate
your browser to the `target/doc/lodil/index.html`._

## [`KeyValueStore`]

The primary structure in the library is called [`KeyValueStore`]. It is essentially a
wrapper around an `Arc<RwLock<HashMap>`, with some methods that attempt to
obtain locks to the underlying `Hashmap` and, when successful, delegate remaining logic
to methods of `HashMap`.

[`KeyValueStore`] exposes three methods for interacting with the underlying [`HashMap`].

 - [`KeyValueStore::insert`] - inserts or updates an entry with an optional expiration time
 - [`KeyValueStore::get`] - gets value associated with the given key, if it exists and has not expired
 - [`KeyValueStore::remove`] - removes entry associated with the given key

For more details on these methods, see their associated documentation.

### Implementation

An [`RwLock`] was used to reduce contention when trying to read from the map. This means
many read locks can be held simultaneously, while only a single write lock can be
held at any time. Additionally, write locks prevent any read locks from being obtained
and vice versa. In other words, write locks create contention.

In conjunction with [`Arc`], the [`KeyValueStore`] can be cloned and moved across thread
boundaries. This is because cloning an [`Arc`] clones a reference to the underlying 
`HashMap` and increments the reference count, which in turn prevents it from being dropped
while still in use.

In this sense, [`KeyValueStore`] is thread safe, as reads and writes to the underlying
structure are synchronized.

#### Memory Usage

There is not a tremendous amount of overhead above and beyond the keys and values themselves.
The space complexity is roughly `O(n)`, where `n` is the number of entries. This excludes heap
allocated space that keys or values may reference, but instead includes only the size of those
references themselves.

Note that expired entries are lazily removed from the underlying `HashMap`, i.e. _expired
entries are removed on read_. This is problematic, as it can essentially create something akin to
a memory leak, if the user isn't diligent about checking keys regularly. This really isn't
ideal, and perhaps a function for clearing expired entries should be written. On the other hand,
it's really simple! It means there's no "garbage collector" thread that has to run periodically.
Perhaps there's some other option I'm not considering?

Additionally, calls to [`KeyValueStore::get`] return clones of the value. This also is not ideal,
especially with large values, but it is much simpler to implement than trying to manage references.

#### Performance Characteristics

In general, calls to each of the methods in [`KeyValueStore`] should exhibit a runtime akin
to analogous functions in [`HashMap`] given that they forward calls the underlying structure.
Each of these calls should execute with theoretical complexities of `O(1)` on average.
However, methods in [`KeyValueStore`] are also dealing with contention, which is hard to predict
without context.

In a context where many reads are happening, presumably the primary method being called will be
[`KeyValueStore::get`], which generally creates no contention for other calls to [`KeyValueStore::get`].
However, when trying to read entries that have expired, calls to [`KeyValueStore::get`] attempt to
obtain a write lock. So, if there are many transient entries, this will increase contention.
That being said, when used for reading only and entries are non-transient, performance will be on the
roughly on the order of the underlying [`HashMap`], but likely worse given that clones are returned
rather than references.

In a context where many writes are happening, i.e. insertions, updates, removals, expiring entries, etc,
there will be significant contention and will exhibit a significant slowdown in read time, i.e. calls
to [`KeyValueStore::get`]. One way to improve this is by using sharding techniques to lock only parts of the
underlying structure, i.e. updating an entry need not prevent reading or updating some other entry.
