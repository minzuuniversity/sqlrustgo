-- BustubX-EDU B-track core seed (ALPHA_TO_BETA_GATE_PLAN AB-05 / #5114)
-- Shared curriculum dataset; exercises build on top and MUST NOT reuse these
-- table names (seed + exercise are concatenated for oracle generation).
CREATE TABLE seed_students (id INTEGER PRIMARY KEY, name TEXT, score INTEGER);
INSERT INTO seed_students VALUES (1, 'ada', 91), (2, 'bob', 74), (3, 'cy', 85), (4, 'dee', 62), (5, 'eve', 77);
CREATE TABLE seed_courses (id INTEGER PRIMARY KEY, title TEXT);
INSERT INTO seed_courses VALUES (10, 'systems'), (20, 'databases'), (30, 'math');
CREATE TABLE seed_enroll (student_id INTEGER, course_id INTEGER);
INSERT INTO seed_enroll VALUES (1, 10), (1, 20), (2, 20), (3, 10), (3, 30), (4, 20), (5, 30);
