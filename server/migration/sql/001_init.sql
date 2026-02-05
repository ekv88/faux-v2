-- phpMyAdmin SQL Dump
-- version 5.2.1
-- https://www.phpmyadmin.net/
--
-- Host: 127.0.0.1:3306
-- Generation Time: Feb 05, 2026 at 07:25 PM
-- Server version: 8.2.0
-- PHP Version: 8.2.13

SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
START TRANSACTION;
SET time_zone = "+00:00";

--
-- Database: `faux`
--

-- --------------------------------------------------------

--
-- Table structure for table `links`
--

DROP TABLE IF EXISTS `links`;
CREATE TABLE IF NOT EXISTS `links` (
  `id` char(36) COLLATE utf8mb4_bin NOT NULL,
  `user_id` char(36) COLLATE utf8mb4_bin DEFAULT NULL,
  `code` varchar(64) COLLATE utf8mb4_bin DEFAULT NULL,
  `pin` char(4) COLLATE utf8mb4_bin DEFAULT NULL,
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `keys`
--

DROP TABLE IF EXISTS `keys`;
CREATE TABLE IF NOT EXISTS `keys` (
  `id` char(36) NOT NULL,
  `user_id` char(36) DEFAULT NULL,
  `name` varchar(64) DEFAULT NULL,
  `key` varchar(512) NOT NULL,
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `packages`
--

DROP TABLE IF EXISTS `packages`;
CREATE TABLE IF NOT EXISTS `packages` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `name` varchar(100) COLLATE utf8mb4_bin NOT NULL,
  `rate_limit` int NOT NULL DEFAULT '0',
  `c_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `payments`
--

DROP TABLE IF EXISTS `payments`;
CREATE TABLE IF NOT EXISTS `payments` (
  `id` char(36) COLLATE utf8mb4_bin NOT NULL,
  `transaction_id` varchar(128) COLLATE utf8mb4_bin DEFAULT NULL,
  `currency` varchar(8) COLLATE utf8mb4_bin DEFAULT NULL,
  `price` decimal(10,2) DEFAULT NULL,
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `roles`
--

DROP TABLE IF EXISTS `roles`;
CREATE TABLE IF NOT EXISTS `roles` (
  `id` char(36) COLLATE utf8mb4_bin NOT NULL,
  `user_id` char(36) COLLATE utf8mb4_bin DEFAULT NULL,
  `name` varchar(64) COLLATE utf8mb4_bin DEFAULT NULL,
  `elevation` int DEFAULT '0',
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `settings`
--

DROP TABLE IF EXISTS `settings`;
CREATE TABLE IF NOT EXISTS `settings` (
  `id` char(36) NOT NULL,
  `user_id` char(36) DEFAULT NULL,
  `name` varchar(64) DEFAULT NULL,
  `config` json DEFAULT NULL,
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `screen_results`
--

DROP TABLE IF EXISTS `screen_results`;
CREATE TABLE IF NOT EXISTS `screen_results` (
  `id` char(36) COLLATE utf8mb4_bin NOT NULL,
  `user_id` char(36) COLLATE utf8mb4_bin DEFAULT NULL,
  `file_name` varchar(255) COLLATE utf8mb4_bin NOT NULL,
  `debug` json DEFAULT NULL,
  `c_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `status` enum('RUNNING','DONE','ERROR') COLLATE utf8mb4_bin NOT NULL DEFAULT 'RUNNING',
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `subscriptions`
--

DROP TABLE IF EXISTS `subscriptions`;
CREATE TABLE IF NOT EXISTS `subscriptions` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `user_id` char(36) COLLATE utf8mb4_bin DEFAULT NULL,
  `payment_id` char(36) COLLATE utf8mb4_bin DEFAULT NULL,
  `package_id` bigint DEFAULT NULL,
  `expires_at` datetime DEFAULT NULL,
  `credits` int NOT NULL DEFAULT '0',
  `c_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- --------------------------------------------------------

--
-- Table structure for table `users`
--

DROP TABLE IF EXISTS `users`;
CREATE TABLE IF NOT EXISTS `users` (
  `id` char(36) COLLATE utf8mb4_bin NOT NULL,
  `email` varchar(190) COLLATE utf8mb4_bin NOT NULL,
  `password` varchar(190) COLLATE utf8mb4_bin NOT NULL,
  `first_name` varchar(100) COLLATE utf8mb4_bin DEFAULT NULL,
  `last_name` varchar(100) COLLATE utf8mb4_bin DEFAULT NULL,
  `confirmd` tinyint(1) NOT NULL DEFAULT '0',
  `c_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `e_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
COMMIT;
