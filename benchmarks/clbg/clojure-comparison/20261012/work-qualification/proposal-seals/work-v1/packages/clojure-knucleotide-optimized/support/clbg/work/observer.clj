(ns clbg.work.observer
  (:require [clojure.java.io :as io] [clojure.string :as string])
  (:import [java.util IdentityHashMap HashMap] [java.lang.reflect Array Field]))

;; Additive observer-only source. It receives real scored objects/operands;
;; tree creation/checking use the immutable original scored compiled namespace;
;; observed histogram writes and division stay in the reviewed main copies.
(def ^:dynamic *worker* 0)
(def identities (IdentityHashMap.))
(def serial (atom 0))
(def state (atom {:graphs [] :intervals [] :joins [] :maps {} :map-order [] :extractions [] :digits []}))
(def trace-lock (Object.))
(def writer (delay (io/writer (System/getenv "CLBG_WORK_TRACE") :encoding "UTF-8")))
(defn object-id [value]
  (locking identities
    (or (.get identities value)
        (let [identity (swap! serial inc)] (.put identities value identity) identity))))
(defn json-string [value]
  (str "\"" (apply str (map (fn [character]
      (case character \" "\\\"" \\ "\\\\" \newline "\\n" \return "\\r" \tab "\\t"
        (if (< (int character) 32) (format "\\u%04x" (int character)) (str character)))) value)) "\""))
(declare json)
(defn json [value]
  (cond
    (nil? value) "null"
    (string? value) (json-string value)
    (keyword? value) (json-string (name value))
    (number? value) (str value)
    (boolean? value) (str value)
    (map? value) (str "{" (string/join "," (map (fn [[key item]] (str (json (if (keyword? key) (name key) key)) ":" (json item))) value)) "}")
    (sequential? value) (str "[" (string/join "," (map json value)) "]")
    :else (throw (IllegalArgumentException. (str "Unreviewed observer JSON type " (class value))))))
(defn emit [kind value]
  (locking trace-lock
    (let [destination @writer]
      (.write destination (json {:kind kind :value value}))
      (.write destination "\n")
      (.flush destination))))
(defn with-worker [worker function]
  (binding [*worker* worker] (function)))
(defn start-interval [depth begin end]
  (let [record {:depth depth :begin begin :end end :worker *worker*
                :thread_id (.threadId (Thread/currentThread)) :event "started"}]
    (locking trace-lock
      (swap! state update :intervals conj record)
      (emit "interval" record))))
(defn end-interval [depth begin end]
  (let [record {:depth depth :begin begin :end end :worker *worker*
                :thread_id (.threadId (Thread/currentThread)) :event "finished"}]
    (locking trace-lock
      (swap! state update :intervals conj record)
      (emit "interval" record))))
(defn join-values [futures values]
  (let [record {:futures (mapv (fn [future] {:object_id (object-id future) :done (.isDone future)}) futures)
                :returned_object_ids (mapv object-id values)
                :returned_numbers (mapv #(when (number? %) %) values)
                :thread_id (.threadId (Thread/currentThread)) :all_gets_completed true}]
    (locking trace-lock
      (swap! state update :joins conj record)
      (emit "joins" record))
    values))
(defn tree-root [root depth kind iteration check]
  (let [nodes (atom []) seen (IdentityHashMap.)]
    (letfn [(visit [node]
              (when (.containsKey seen node) (throw (IllegalStateException. "Repeated tree object")))
              (.put seen node true)
              (let [left (aget node 0) right (aget node 1)]
                (swap! nodes conj {:id (object-id node) :left (when left (object-id left))
                    :right (when right (object-id right)) :array_length (alength node)})
                (when left (visit left))
                (when right (visit right))))]
      (visit root))
    (let [record {:root_id (object-id root) :nodes @nodes :depth depth :kind kind
                  :iteration iteration :worker *worker* :thread_id (.threadId (Thread/currentThread))
                  :phase (case kind "retained-before" "before-temporary-work" "retained-after" "after-all-temporary-work" kind)
                  :actual_recursive_check (check root) :actual_checker_class (.getName (class check))}]
      (locking trace-lock
        (swap! state update :graphs conj record)
        (emit "tree" record)))
    root))
(defn field [object name]
  (loop [klass (class object)]
    (when klass
      (let [found (try (.getDeclaredField klass name) (catch NoSuchFieldException _ nil))]
        (if found (do (.setAccessible found true) (.get found object))
          (recur (.getSuperclass klass)))))))
(defn storage-slots [table]
  (if (instance? HashMap table)
    (let [array (field table "table")] (if array (Array/getLength array) 0))
    (let [seen (IdentityHashMap.)]
      (letfn [(slots [node]
                (if (or (nil? node) (.containsKey seen node)) 0
                  (do (.put seen node true)
                    (if (.isArray (class node))
                      (+ (Array/getLength node) (reduce + 0 (map #(slots (Array/get node %)) (range (Array/getLength node)))))
                      (if (string/starts-with? (.getName (class node)) "clojure.lang.PersistentHashMap$")
                        (slots (field node "array")) 0))))) ]
        (slots (field table "root"))))))
(defn map-start [table sequence width]
  (let [record {:width width :sequence sequence :initial_object_id (object-id table)
                :initial_storage_class (.getName (class table)) :initial_count (count table)
                :initial_storage_slots (storage-slots table) :peak_storage_slots (storage-slots table)
                :owner *worker* :thread_id (.threadId (Thread/currentThread)) :private_owner true :actual_updates []}]
    (swap! state assoc-in [:maps width] record)
    (emit "map-start" record)
    table))
(defn map-write [table key position width write]
  (let [old (get table key 0) returned (write)
        actual (if (instance? HashMap table) table returned)
        record {:position position :key key :old old :new (get actual key 0)
                :worker *worker* :thread_id (.threadId (Thread/currentThread))}
        slots (storage-slots actual)]
    (swap! state (fn [value] (-> value (update-in [:maps width :actual_updates] conj record)
                                  (update-in [:maps width :peak_storage_slots] max slots))))
    (emit "map-write" (assoc record :width width :object_id (object-id actual) :storage_slots slots))
    returned))
(defn map-return [table sequence width]
  (let [entries (mapv (fn [[key value]] {:key key :count value}) table)
        record {:returned_object_id (object-id table) :returned_storage_class (.getName (class table))
                :returned_thread_id (.threadId (Thread/currentThread))
                :final_count (count table) :actual_entries entries :peak_storage_slots (storage-slots table)}]
    (swap! state (fn [value] (-> value (update-in [:maps width] merge (dissoc record :peak_storage_slots))
                                  (update-in [:maps width :peak_storage_slots] max (:peak_storage_slots record)))))
    (emit "map-return" (assoc record :width width))
    table))
(defn maps-received [tables]
  (let [ids (mapv object-id tables)]
    (swap! state assoc :map-order ids)
    (doseq [[width table] (map vector [1 2 3 4 6 12 18] tables)]
      (swap! state update-in [:maps width] assoc :received_object_id (object-id table) :parent_read_after_worker_join true))
    (emit "maps-received" {:object_ids ids :thread_id (.threadId (Thread/currentThread))})
    tables))
(defn pi-step [q r t k emitted numerator-three numerator-four three four]
  (let [accepted (.equals three four)
        record {:q q :r r :t t :k k :emitted emitted :numerator_three numerator-three :numerator_four numerator-four
                :divisor_three t :divisor_four t :quotient_three three :quotient_four four :accepted accepted}]
    (swap! state update :extractions conj record)
    (when accepted (swap! state update :digits conj (str (.intValue three))))
    (emit "pi" record)))
(defn finish []
  (let [value @state benchmark (System/getenv "CLBG_WORK_BENCHMARK")
        variant (System/getenv "CLBG_WORK_VARIANT") n (Long/parseLong (System/getenv "CLBG_WORK_N"))
        document (case benchmark
          "binarytrees" {:n n :variant variant :graphs (:graphs value) :interval_events (:intervals value) :worker_joins (:joins value)
                         :retained_root_read_after_all_worker_joins true
                         :actual_total_node_allocations (count (set (map :id (mapcat :nodes (:graphs value)))))}
          "knucleotide" {:sequence (:sequence (get (:maps value) 1)) :variant variant
                         :maps (mapv #(get (:maps value) %) [1 2 3 4 6 12 18])
                         :retained_returned_map_ids (:map-order value) :worker_joins (:joins value)}
          "pidigits" {:wanted n :extractions (:extractions value) :emitted_digits (apply str (:digits value))})]
    (emit "document" document)
    (.close @writer)))
