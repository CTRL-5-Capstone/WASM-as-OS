package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"
	"sync"

	"github.com/gorilla/mux"
	"github.com/rs/cors"
)

type Task struct {
	ID   int    `json:"id"`
	Name string `json:"name"`
}

type Server struct {
	tasks  map[int]*Task
	nextID int
	mu     sync.RWMutex
}

func NewServer() *Server {
	return &Server{
		tasks:  make(map[int]*Task),
		nextID: 1,
	}
}

func (s *Server) getTasks(w http.ResponseWriter, r *http.Request) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	tasks := make([]*Task, 0, len(s.tasks))
	for _, task := range s.tasks {
		tasks = append(tasks, task)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(tasks)
}

func (s *Server) createTask(w http.ResponseWriter, r *http.Request) {
	file, header, err := r.FormFile("file")
	if err != nil {
		http.Error(w, "No file uploaded", http.StatusBadRequest)
		return
	}
	defer file.Close()

	// Read file to validate it's a WASM file
	data, err := io.ReadAll(file)
	if err != nil {
		http.Error(w, "Failed to read file", http.StatusBadRequest)
		return
	}

	// Basic WASM validation (check magic number)
	if len(data) < 4 || string(data[:4]) != "\x00asm" {
		http.Error(w, "Invalid WASM file", http.StatusBadRequest)
		return
	}

	s.mu.Lock()
	task := &Task{
		ID:   s.nextID,
		Name: header.Filename,
	}
	s.tasks[s.nextID] = task
	s.nextID++
	s.mu.Unlock()

	log.Printf("Started task %d: %s", task.ID, task.Name)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(task)
}

func (s *Server) deleteTask(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.Atoi(vars["id"])
	if err != nil {
		http.Error(w, "Invalid task ID", http.StatusBadRequest)
		return
	}

	s.mu.Lock()
	task, exists := s.tasks[id]
	if !exists {
		s.mu.Unlock()
		http.Error(w, "Task not found", http.StatusNotFound)
		return
	}
	delete(s.tasks, id)
	s.mu.Unlock()

	log.Printf("Stopped task %d: %s", task.ID, task.Name)

	w.WriteHeader(http.StatusNoContent)
}

func main() {
	server := NewServer()

	r := mux.NewRouter()
	api := r.PathPrefix("/api").Subrouter()

	api.HandleFunc("/tasks", server.getTasks).Methods("GET")
	api.HandleFunc("/tasks", server.createTask).Methods("POST")
	api.HandleFunc("/tasks/{id}", server.deleteTask).Methods("DELETE")

	c := cors.New(cors.Options{
		AllowedOrigins: []string{"http://localhost:5173"},
		AllowedMethods: []string{"GET", "POST", "DELETE"},
		AllowedHeaders: []string{"*"},
	})

	handler := c.Handler(r)

	fmt.Println("Server starting on :8080")
	log.Fatal(http.ListenAndServe(":8080", handler))
}