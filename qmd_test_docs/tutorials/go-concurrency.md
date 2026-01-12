# Go Concurrency Patterns

## Goroutines

Lightweight threads managed by the Go runtime:

```go
func main() {
    go sayHello()
    time.Sleep(time.Second)
}

func sayHello() {
    fmt.Println("Hello from goroutine!")
}
```

## Channels

Channels provide communication between goroutines:

```go
func worker(jobs <-chan int, results chan<- int) {
    for job := range jobs {
        results <- job * 2
    }
}

func main() {
    jobs := make(chan int, 100)
    results := make(chan int, 100)

    // Start 3 workers
    for w := 0; w < 3; w++ {
        go worker(jobs, results)
    }

    // Send 5 jobs
    for j := 1; j <= 5; j++ {
        jobs <- j
    }
    close(jobs)

    // Collect results
    for r := 1; r <= 5; r++ {
        fmt.Println(<-results)
    }
}
```

## Select Statement

Handle multiple channel operations:

```go
select {
case msg := <-messages:
    fmt.Println("Received:", msg)
case <-timeout:
    fmt.Println("Timeout!")
default:
    fmt.Println("No activity")
}
```

## WaitGroups

Coordinate multiple goroutines:

```go
var wg sync.WaitGroup

for i := 0; i < 5; i++ {
    wg.Add(1)
    go func(id int) {
        defer wg.Done()
        fmt.Printf("Worker %d done\n", id)
    }(i)
}

wg.Wait()
```

## Common Pitfalls

- **Deadlocks** - All goroutines blocked waiting for each other
- **Race conditions** - Multiple goroutines accessing shared data
- **Channel leaks** - Not closing channels or goroutines blocking forever
