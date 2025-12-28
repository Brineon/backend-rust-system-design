## 📺 Live Demo (No Setup Required)

You can run this full architecture (Rust + Redis + Kafka) in your browser without installing Docker.

1. Click the green **Code** button on this repo.
2. Select the **Codespaces** tab.
3. Click **Create codespace on main**.
4. Wait for the terminal to load, then run:
   ```bash
   docker compose up --build






# RustyScale: High-Performance Async Job System

A production-grade distributed backend built with **Rust**, **Redis**, **Kafka**, and **Nginx**. Designed to demonstrate high concurrency, fault tolerance, and rate limiting.

## Architecture

* **API Gateway:** Nginx (Reverse Proxy + Rate Limiting)
* **Backend:** Rust + Axum (Async/Await)
* **Message Broker:** Redis Pub/Sub
* **Fault Tolerance:** Kafka (Dead Letter Queue)
* **Deployment:** Docker Compose

## Features

* **Async Job Processing:** Decoupled architecture using Redis.
* **Real-Time Updates:** WebSockets for live status tracking.
* **Rate Limiting:** IP-based throttling (10 req/min) via Nginx.
* **Failure Recovery:** Failed jobs are automatically routed to Kafka.
* **Horizontal Scaling:** Stateless backend design allows easy replication.
* **List Jobs API:** Endpoint to retrieve status of all active jobs.

## Quick Start

1. **Run the System:**
```bash
docker compose up --build

```


2. **Create a Job:**
```bash
curl -X POST http://localhost/createjob

```


3. **Test Rate Limiting:**
Run a loop to see Nginx block traffic:
```bash
for i in {1..20}; do curl -X POST http://localhost/createjob; done

```


4. **Verify Kafka Failures:**
Check the Dead Letter Queue for failed jobs:
```bash
docker compose exec kafka kafka-console-consumer --bootstrap-server kafka:9092 --topic failed_jobs --from-beginning

```
